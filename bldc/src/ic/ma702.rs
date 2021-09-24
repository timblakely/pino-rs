//! Implementation of the MA702 12-bit angle sensor.

use crate::block_until;
use crate::block_while;
use crate::util::buffered_state::{BufferedState, StateReader, StateWriter};
use crate::util::stm32::blocking_sleep_us;
use core::f32::consts::PI;
use stm32g4::stm32g474::{self as device, interrupt};
use third_party::m4vga_rs::util::armv7m::{clear_pending_irq, enable_irq};
use third_party::m4vga_rs::util::spin_lock::SpinLock;
use third_party::m4vga_rs::util::sync::acquire_hw;

const FREQ_HZ: f32 = 1000.;
const TWO_PI: f32 = 2. * PI;

// Static location in memory to stream the raw angle measurements to. This has to be a) in a
// consistent location and b) In RAM, not flash.
pub static mut ANGLE: u16 = 0;

#[derive(Clone, Copy)]
pub struct AngleState {
    pub raw_angle: Option<u16>,
    pub angle: f32, // Radians
    pub velocity: f32,
    pub acceleration: f32,
}

pub struct Ma702<S> {
    // TODO(blakely): Make generic via a trait or enum.
    spi: device::SPI1,
    tim3: device::TIM3,

    #[allow(dead_code)]
    mode_state: S,
}

pub struct Init {}
pub struct Ready {}

pub struct Streaming {
    state: StateReader<AngleState>,
}

pub fn new(spi: device::SPI1, tim3: device::TIM3) -> Ma702<Init> {
    Ma702 {
        spi,
        tim3,
        mode_state: Init {},
    }
}

impl Ma702<Init> {
    pub fn configure_spi(self) -> Ma702<Ready> {
        // SPI config
        let spi1 = self.spi;

        // Disable SPI, if enabled.
        spi1.cr1.modify(|_, w| w.spe().clear_bit());
        block_until! { spi1.cr1.read().spe().bit_is_clear() }
        // Idle clock low, data capture on rising edge, transmission on falling edge
        // TODO(blakely): This assumes that the processor is running full bore at 170MHz
        spi1.cr1.modify(|_, w| {
            w.cpha()
                .clear_bit()
                .cpol()
                .clear_bit()
                .mstr()
                .set_bit()
                .br()
                .div128()
                .crcen()
                .clear_bit()
        });
        // 16 bit transfers
        spi1.cr2.modify(|_, w| {
            w.ssoe()
                .enabled()
                .frf()
                .clear_bit()
                .ds()
                .sixteen_bit()
                .nssp()
                .set_bit()
        });

        Ma702 {
            spi: spi1,
            tim3: self.tim3,
            mode_state: Ready {},
        }
    }
}

impl Ma702<Ready> {
    fn configure_tx_stream(&self, dma: &device::DMA1, dmamux: &device::DMAMUX) {
        // Configure DMA1 stream 1 to transfer a `0` into `SPI1[DR]` to trigger an SPI transaction,
        // off the update event from tim3.

        // Disable DMA channel if it's enabled.
        dma.ccr1.modify(|_, w| w.en().clear_bit());
        block_until!(dma.ccr1.read().en().bit_is_clear());
        // Configure for memory-to-peripheral mode @ 16-bit. Don't change address for either memory
        // or peripheral.
        dma.ccr1.modify(|_, w| unsafe {
            // Safety: Upstream: This should be a 2-bit enum. 0b01 = 16-bit
            w.msize()
                .bits(0b01)
                // Safety: Upstream: This should be a 2-bit enum. 0b01 = 16-bit
                .psize()
                .bits(0b01)
                .minc()
                .clear_bit()
                .pinc()
                .clear_bit()
                .circ()
                .set_bit()
                .dir()
                .set_bit()
        });
        // Just transfer a single value
        // Safety: Upstream: This should have a proper range of 0-65535 in stm32-rs.
        dma.cndtr1.write(|w| unsafe { w.ndt().bits(1) });
        // Target memory location
        {
            pub static mut MA702_REQUEST_ANGLE: u16 = 0;
            // Safety: This is the source of the DMA stream. We've configured it for 16-bit
            // and the address we're taking is a `u16`
            dma.cmar1
                .write(|w| unsafe { w.ma().bits(((&MA702_REQUEST_ANGLE) as *const _) as u32) });
        }
        // Target peripheral location
        {
            let spi = &self.spi;
            // Safety: Erm... its not? XD We're asking the DMA to stream data to an arbitrary
            // address, which is in no way shape or form safe. We've set it up so that it's a `u16`
            // transfer from the static above to `SPI[DR]`. YOLO
            dma.cpar1
                .write(|w| unsafe { w.pa().bits(((&spi.dr) as *const _) as u32) });
        }

        // Now we wire up the DMA triggers to their respective streams
        // Note: DMAMUX channels 0-7 connected to DMA1 channels 1-8, 8-15=DMA2 1-8
        // TIM3 Update to the DMA stream 1 - TIM3_UP = 65
        // Safety: Upstream: This should be an enum.
        // TODO(blakely): Add enum values to `stm32-rs`
        dmamux.c0cr.modify(|_, w| unsafe { w.dmareq_id().bits(65) });
    }

    fn configure_rx_stream(&self, dma: &device::DMA1, dmamux: &device::DMAMUX) {
        // Configure DMA1 stream 2 to read from `SPI1[DR]` and stream to ANGLE on update from TIM3.

        // Disable DMA channel if it's enabled.
        dma.ccr2.modify(|_, w| w.en().clear_bit());
        block_until!(dma.ccr2.read().en().bit_is_clear());
        // Configure for peripheral-to-memory mode @ 16-bit. Don't change address for either memory
        // or peripheral.
        dma.ccr2.modify(|_, w| unsafe {
            // Safety: Upstream: This should be a 2-bit enum. 0b01 = 16-bit
            w.msize()
                .bits(0b01)
                // Safety: Upstream: This should be a 2-bit enum. 0b01 = 16-bit
                .psize()
                .bits(0b01)
                .minc()
                .clear_bit()
                .pinc()
                .clear_bit()
                .circ()
                .set_bit()
                // Peripheral-to-Memory this time.
                .dir()
                .clear_bit()
        });
        // Just transfer a single value
        // Safety: Upstream: This should have a proper range of 0-65535 in stm32-rs.
        dma.cndtr2.write(|w| unsafe { w.ndt().bits(1) });
        // Target memory location
        {
            // Safety: This is the destination of the DMA stream. We've configured it for 16-bit
            // and the address we're taking is a `u16`. We're also taking a reference to a `static
            // mut` which is normally bad, but DMA writes _should_ be atomic.
            dma.cmar2
                .write(|w| unsafe { w.ma().bits(((&ANGLE) as *const _) as u32) });
        }
        // Target peripheral location
        {
            let spi = &self.spi;
            // Safety: We're reading from an arbitrary location in memory: the data register of the
            // SPI peripheral. It's configured to read 16 bits, the width of the packet we're
            // requesting from the SPI peripheral.
            dma.cpar2
                .write(|w| unsafe { w.pa().bits(((&spi.dr) as *const _) as u32) });
        }

        // Now we wire up the DMA triggers to their respective streams
        // Note: DMAMUX channels 0-7 connected to DMA1 channels 1-8, 8-15=DMA2 1-8
        // TIM3 Update to the DMA stream2 - TIM3_UP = 65
        // Safety: Upstream: This should be an enum.
        // TODO(blakely): Add enum values to `stm32-rs`
        dmamux.c1cr.modify(|_, w| unsafe { w.dmareq_id().bits(65) });
    }

    pub fn begin_stream(self, dma: device::DMA1, dmamux: &device::DMAMUX) -> Ma702<Streaming> {
        self.configure_tx_stream(&dma, dmamux);
        self.configure_rx_stream(&dma, dmamux);

        let mut angle_state = MA702_STATE.lock();
        *angle_state = Some(BufferedState::new(AngleState {
            raw_angle: None,
            angle: 0.,
            acceleration: 0.,
            velocity: 0.,
        }));
        let (reader, writer) = angle_state.as_mut().expect("asdf").split();
        *MA702_STATE_WRITER.lock() = Some(writer);

        // Enable SPI.
        self.spi.cr1.modify(|_, w| w.spe().set_bit());
        block_until! { self.spi.cr1.read().spe().bit_is_set() }

        // Enable DMA stream 1.
        dma.ccr1.modify(|_, w| w.en().set_bit());
        block_until! {  dma.ccr1.read().en().bit_is_set() }

        // Enable DMA stream 2.
        dma.ccr2.modify(|_, w| w.en().set_bit());
        block_until! {  dma.ccr2.read().en().bit_is_set() }

        // Kick off tim3 to start the stream.
        self.tim3.cr1.modify(|_, w| w.cen().set_bit());
        // Wait a bit to ensure there's no garbage coming across SPI
        blocking_sleep_us(5000);

        // Enable the DMA1[CH2] Transfer Complete interrupt so that the handler is called when the
        // transfer from SPI to memory is complete.
        dma.ccr2.modify(|_, w| w.tcie().set_bit());

        // Make sure we drop the lock before we enable the IRQ
        {
            // Donate the DMA so that the interrupt handler can clear the interrupt flag.
            *DMA1.lock() = Some(dma);
        }

        // Enable the DMA1[CH2] interrupt in NVIC.
        enable_irq(device::Interrupt::DMA1_CH2);

        Ma702 {
            spi: self.spi,
            tim3: self.tim3,
            mode_state: Streaming { state: reader },
        }
    }
}

// TODO(blakely): Combine this and the state writer into a single lock.
pub static DMA1: SpinLock<Option<device::DMA1>> = SpinLock::new(None);

pub static MA702_STATE: SpinLock<Option<BufferedState<AngleState>>> = SpinLock::new(None);
pub static MA702_STATE_WRITER: SpinLock<Option<StateWriter<AngleState>>> = SpinLock::new(None);

// This is the interrupt that fires when the transfer from the `SPI[DR]` register transaction is complete.
#[interrupt]
fn DMA1_CH2() {
    // Clear pending IRQ in NVIC.
    clear_pending_irq(device::Interrupt::DMA1_CH2);

    let mut state = acquire_hw(&MA702_STATE_WRITER);
    let mut state = state.update();
    let raw_angle = unsafe { ANGLE >> 4 };
    let angle = raw_angle as f32 / 4096. * TWO_PI;

    let (last_angle, last_velocity) = match state.other().raw_angle {
        None => (angle, 0f32),
        Some(_) => {
            let other = state.other();
            (other.angle, other.velocity)
        }
    };

    let velocity = (angle - last_angle) / FREQ_HZ;
    let acceleration = (state.velocity - last_velocity) / FREQ_HZ;

    *state = AngleState {
        raw_angle: Some(raw_angle),
        angle,
        velocity,
        acceleration,
    };

    // Finally clear the IRQ flag in the DMA itself.
    let dma = acquire_hw(&DMA1);
    dma.ifcr.write(|w| w.gif2().set_bit());
}
