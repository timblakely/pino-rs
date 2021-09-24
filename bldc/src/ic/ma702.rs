//! Implementation of the MA702 12-bit angle sensor.

use crate::block_until;
use crate::block_while;
use crate::util::buffered_state::{BufferedState, StateReader, StateWriter};
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
    pub turns: u32,
    pub angle_multiturn: f32,
}

impl AngleState {
    pub fn new() -> AngleState {
        AngleState {
            raw_angle: None,
            angle: 0.,
            acceleration: 0.,
            velocity: 0.,
            turns: 0,
            angle_multiturn: 0.,
        }
    }
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

pub struct StreamingPolling {
    state: AngleState,
}

pub struct StreamingInterrupt {
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
                .div64()
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
                // Enable RX DMA trigger
                .rxdmaen()
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
        dmamux.c1cr.modify(|_, w| unsafe { w.dmareq_id().bits(10) });
    }

    fn start_stream(&mut self, dma: &device::DMA1, dmamux: &device::DMAMUX) {
        self.configure_tx_stream(dma, dmamux);
        self.configure_rx_stream(dma, dmamux);

        // Enable SPI.
        self.spi.cr1.modify(|_, w| w.spe().set_bit());
        block_until! { self.spi.cr1.read().spe().bit_is_set() }

        // Enable DMA stream 1.
        dma.ccr1.modify(|_, w| w.en().set_bit());
        block_until! {  dma.ccr1.read().en().bit_is_set() }

        // Enable DMA stream 2.
        dma.ccr2.modify(|_, w| w.en().set_bit());
        block_until! {  dma.ccr2.read().en().bit_is_set() }

        // Configure TIM3 for 1kHz polling of SPI1
        let tim3 = &self.tim3;
        // Stop the timer if it's running for some reason.
        tim3.cr1.modify(|_, w| w.cen().clear_bit());
        block_until!(tim3.cr1.read().cen().bit_is_clear());
        // Edge aligned mode, and up counting.
        tim3.cr1.modify(|_, w| w.dir().up().cms().edge_aligned());
        // Fire off a DMA on update (i.e. counter overflow)
        tim3.dier.modify(|_, w| w.ude().set_bit());
        // Assuming 170MHz core clock, set prescalar to 4 and ARR to 42500 for 170e6/42500/4=1kHz.
        // Why is the value actually 3 and not 4? The timer clock is set to `core_clk / (PSC[PSC] +
        // 1)`. If it were to use the value directly it'd divide the clock by zero on reset, which
        // would be A Bad Thing.
        // Safety: Upstream: This should have a proper range of 0-65535 in stm32-rs. 3 is well
        // within range.
        tim3.psc.write(|w| w.psc().bits(3));
        // Safety: Upstream: This should have a proper range of 0-65535 in stm32-rs. 42500 is within
        // range.
        tim3.arr.write(|w| unsafe { w.arr().bits(425) });
        // Kick off tim3 to start the stream.
        tim3.cr1.modify(|_, w| w.cen().set_bit());
    }

    pub fn begin_stream_polling(
        mut self,
        dma: device::DMA1,
        dmamux: &device::DMAMUX,
    ) -> Ma702<StreamingPolling> {
        self.start_stream(&dma, dmamux);

        Ma702 {
            spi: self.spi,
            tim3: self.tim3,
            mode_state: StreamingPolling {
                state: AngleState::new(),
            },
        }
    }

    pub fn begin_stream_interrupt(
        mut self,
        dma: device::DMA1,
        dmamux: &device::DMAMUX,
    ) -> Ma702<StreamingInterrupt> {
        self.start_stream(&dma, dmamux);

        let mut angle_state = MA702_STATE.lock();
        *angle_state = Some(BufferedState::new(AngleState::new()));

        // Enable the DMA1[CH2] Transfer Complete interrupt so that the handler is called when the
        // transfer from SPI to memory is complete.
        dma.ccr2.modify(|_, w| w.tcie().set_bit());

        let (reader, writer) = angle_state
            .as_mut()
            .expect("Cannot acquire MA702 state")
            .split();

        // Clear DMA IRQ flag.
        dma.ifcr.write(|w| w.gif2().set_bit());

        // Donate the DMA so that the interrupt handler can clear the interrupt flag, and the writer
        // so that it can update the sensor's state.
        *MA702_INTERRUPT_DATA.lock() = Some((dma, writer));

        // Enable the DMA1[CH2] interrupt in NVIC.
        enable_irq(device::Interrupt::DMA1_CH2);

        Ma702 {
            spi: self.spi,
            tim3: self.tim3,
            mode_state: StreamingInterrupt { state: reader },
        }
    }
}

impl Ma702<StreamingInterrupt> {
    pub fn state(&self) -> &AngleState {
        self.mode_state.state.read()
    }
}

impl Ma702<StreamingPolling> {
    pub fn update(&mut self, delta_t: f32) -> AngleState {
        let new_state = calculate_new_angle_state(&self.mode_state.state, delta_t);
        self.mode_state.state = new_state;
        new_state
    }
}

pub static MA702_STATE: SpinLock<Option<BufferedState<AngleState>>> = SpinLock::new(None);
pub static MA702_INTERRUPT_DATA: SpinLock<Option<(device::DMA1, StateWriter<AngleState>)>> =
    SpinLock::new(None);

// Read the global angle value being streamed to by the DMA and return both the raw angle and the
// angle calculated in radians.
fn read_angle() -> (u16, f32) {
    // Safety: accessing global mutable values is inherently unsafe. Technically ANGLE doesn't need
    // to be mutable since no user code is mutating it, but it _is_ modified by the DMA controller,
    // so better safe than sorry. That said, read access to it should be atomic and take only a
    // single instruction.
    let raw_angle = unsafe { ANGLE >> 4 };
    let angle = raw_angle as f32 / 4096. * TWO_PI;
    (raw_angle, angle)
}

// TODO(blakely): Handle non-ero offsets and correct for negative angle values (keep between 0 and
// 2pi)
fn calculate_new_angle_state(old_state: &AngleState, delta_t: f32) -> AngleState {
    let (raw_angle, angle) = read_angle();
    // Special case when it's the very first reading to protect against first-sample velocity.
    let (last_angle, last_velocity) = match old_state.raw_angle {
        None => (angle, 0f32),
        Some(_) => {
            let other = old_state;
            (other.angle, other.velocity)
        }
    };

    let turns = old_state.turns;
    let d_angle = match angle - last_angle {
        d_angle if d_angle > PI => {
            // Rolled over backwards.
            turns -= 1;
            d_angle - TWO_PI
        }
        d_angle if d_angle < -PI => {
            // Rolled over forwards.
            turns += 1;
            d_angle - TWO_PI
        }
        d_angle => d_angle,
    };

    let velocity = d_angle * delta_t;
    let acceleration = (velocity - last_velocity) * delta_t;

    AngleState {
        raw_angle: Some(raw_angle),
        angle,
        velocity,
        acceleration,
        turns,
        angle_multiturn: turns as f32 * TWO_PI + angle,
    }
}

// This is the interrupt that fires when the transfer from the `SPI[DR]` register transaction is
// complete.
#[interrupt]
fn DMA1_CH2() {
    unsafe {
        *(0x4800_0418 as *mut u32) = 1 << 6;
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        cortex_m::asm::nop();
        *(0x4800_0418 as *mut u32) = 1 << (6 + 16);
    }
    // Clear pending IRQ in NVIC.
    clear_pending_irq(device::Interrupt::DMA1_CH2);

    let (dma, ref mut state) = &mut *acquire_hw(&MA702_INTERRUPT_DATA);
    let mut state = state.update();
    // Get a reference to the state that's not being read.
    let last_state = state.other();
    const DELTA_T: f32 = 1. / FREQ_HZ;
    *state = calculate_new_angle_state(&last_state, DELTA_T);
    // Clear DMA IRQ flag.
    dma.ifcr.write(|w| w.gif2().set_bit());
}
