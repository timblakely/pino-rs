//! Implementation of the MA702 12-bit angle sensor.

use crate::block_until;
use crate::block_while;
use crate::util::buffered_state::{BufferedState, StateReader, StateWriter};
use stm32g4::stm32g474 as device;
use third_party::m4vga_rs::util::armv7m::enable_irq;
use third_party::m4vga_rs::util::spin_lock::SpinLock;
use third_party::m4vga_rs::util::sync::acquire_hw;

// Static location in memory to stream the raw angle measurements to. This has to be a) in a
// consistent location and b) In RAM, not flash.
pub static mut ANGLE: u16 = 0;

#[derive(Clone, Copy)]
pub struct AngleState {
    pub raw_angle: u16,
    pub angle: f32, // Radians
    pub angular_velocity: f32,
    pub angular_acceleration: f32,
    pub last_angle: Option<f32>,
    pub last_velocity: Option<f32>,
}

pub struct Ma702<S> {
    // TODO(blakely): Make generic via a trait or enum.
    spi: device::SPI1,

    #[allow(dead_code)]
    mode_state: S,

    state: BufferedState<'static, AngleState>,
}

pub struct Init {}
pub struct Ready {}

pub struct Streaming {}

pub fn new(spi: device::SPI1) -> Ma702<Init> {
    Ma702 {
        spi,
        mode_state: Init {},
        state: BufferedState::new(AngleState {
            raw_angle: 0,
            angle: 0.,
            angular_acceleration: 0.,
            angular_velocity: 0.,
            last_angle: None,
            last_velocity: None,
        }),
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
            mode_state: Ready {},
            state: self.state,
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

    pub fn begin_stream(mut self, dma: device::DMA1, dmamux: &device::DMAMUX) -> Ma702<Streaming> {
        self.configure_tx_stream(&dma, dmamux);
        self.configure_rx_stream(&dma, dmamux);

        let (reader, writer) = self.state.split();

        *MA702_STATE_READER.lock() = Some(reader);
        *MA702_STATE_WRITER.lock() = Some(writer);

        // Enable SPI.
        self.spi.cr1.modify(|_, w| w.spe().set_bit());
        block_until! { self.spi.cr1.read().spe().bit_is_set() }

        // Enable DMA stream 1.
        dma.ccr1.modify(|_, w| w.en().set_bit());
        block_until! {  dma.ccr1.read().en().bit_is_set() }

        // Enable the DMA1[CH2] interrupt in NVIC.
        enable_irq(device::Interrupt::DMA1_CH2);
        // Enable the DMA1[CH2] Transfer Complete interrupt so that the handler is called when the
        // transfer from SPI to memory is complete.
        dma.ccr2.modify(|_, w| w.tcie().set_bit());

        // Enable DMA stream 2.
        dma.ccr2.modify(|_, w| w.en().set_bit());
        block_until! {  dma.ccr2.read().en().bit_is_set() }

        // Donate the DMA so that the interrupt handler can clear the interrupt flag.
        *DMA1.lock() = Some(dma);

        Ma702 {
            spi: self.spi,
            mode_state: Streaming {},
            state: self.state,
        }
    }
}

// TODO(blakely): Combine this and the state writer into a single lock.
pub static DMA1: SpinLock<Option<device::DMA1>> = SpinLock::new(None);

pub static MA702_STATE_READER: SpinLock<Option<StateReader<AngleState>>> = SpinLock::new(None);
pub static MA702_STATE_WRITER: SpinLock<Option<StateWriter<AngleState>>> = SpinLock::new(None);
