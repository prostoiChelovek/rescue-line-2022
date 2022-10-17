#![no_main]
#![no_std]
#![feature(let_chains)]

use panic_probe as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [EXTI1, EXTI2, EXTI3])]
mod app {
    use core::ops;

    use embedded_hal::{digital::v2::OutputPin, blocking::{i2c, delay::DelayMs}};
    use rtt_target::{rtt_init_print, rprintln, rprint};

    use stm32f4xx_hal::{
        prelude::*, pac, pac::USART2,
        block, delay::Delay,
        timer, timer::{monotonic::MonoTimer, Timer},
        gpio, gpio::{
            gpioa::{PA10, PA9, PA8}, gpiob::{PB3, PB10, PB4, PB5},
            gpiob,
            Output, PushPull, gpioc::PC7, Input, PullUp,
        },
        i2c::I2c,
        serial, serial::{config::Config, Event, Serial}, pwm::PwmChannel,
    };
    use fugit::{RateExtU32, HertzU32, Duration};

    use numtoa::NumToA;

    use heapless::Vec;
    use array_init::from_iter;

    use itertools::Itertools;

    use stepper::{Stepper, StepperDireciton};

    const NUM_SENSORS: usize = 6;

    const EDGE_THRESHOLD: u16 = 110;

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimer<pac::TIM2, 1_000_000>;

    type I2cType = I2c<
        pac::I2C1,
        (gpiob::PB8<gpio::Alternate<gpio::OpenDrain, 4>>,
         gpiob::PB9<gpio::Alternate<gpio::OpenDrain, 4>>),
         >;

    pub fn std_dev(vals: impl ExactSizeIterator<Item = u16> + Clone) -> u32 {
        let len = vals.len();
        let mean = vals.clone().sum::<u16>() / len as u16;
        vals
            .map(|x| ((x as i32 - mean as i32).pow(2)) as u32)
            .sum::<u32>() / len as u32
    }

    pub struct LineSensor<BUS: i2c::Write + i2c::WriteRead> {
        bus: BUS,
        sens: u8,
        correction: [i32; NUM_SENSORS]
    }

    impl<BUS, E> LineSensor<BUS>
    where
        BUS: i2c::Write<Error = E> + i2c::WriteRead<Error = E>,
        E: core::fmt::Debug
    {
        pub fn new(bus: BUS) -> Self {
            let mut res = Self {
                bus,
                sens: 0,
                correction: [0; NUM_SENSORS]

            };

            // TODO: not the best practive to do this in constructor
            res.into_output(0); // sens
            res.into_output(9); // IR leds
            for pin in 1..=8 {
                res.into_input(pin);
            }
            res.digital_write(9, true); // enable the IR leds

            res
        }

        fn map_pin(pin: u8) -> u8 {
            // who the fuck came up with this shit numbering scheme
            [0, 4, 5, 6, 8, 7, 3, 2, 1, 9][pin as usize]
        }

        pub fn into_input(&mut self, pin: u8) {
            let pin = Self::map_pin(pin);

            let raw_pin_num = (1u16 << pin as u16).to_be_bytes();
            self.bus.write(
                0x2a,
                &[
                0x04,
                raw_pin_num[0],
                raw_pin_num[1],
                ],
                ).unwrap();
        }

        pub fn into_output(&mut self, pin: u8) {
            let pin = Self::map_pin(pin);

            let raw_pin_num = (1u16 << pin as u16).to_be_bytes();
            self.bus.write(
                0x2a,
                &[
                0x07,
                raw_pin_num[0],
                raw_pin_num[1],
                ],
                ).unwrap();
        }

        pub fn analog_read(&mut self, pin: u8) -> u16 {
            let pin = Self::map_pin(pin);

            let mut inner = || {
                let write_buf = [0x0C, pin];
                let mut read_buf = [0u8; 2];

                self.bus
                    .write_read(0x2a, &write_buf, &mut read_buf)
                    .unwrap();

                u16::from_be_bytes(read_buf)
            };

            // TODO: a hack to get around a bug that causes the sensor get output a value for the
            //       previously requested pin
            inner();
            inner()
        }

        pub fn analog_write(&mut self, pin: u8, val: u16) {
            let raw_value: [u8; 2] = val.to_be_bytes();

            self.bus.write(
                0x2a,
                &[
                0x0B,
                pin,
                raw_value[0],
                raw_value[1],
                ],
                ).unwrap();
        }

        pub fn digital_write(&mut self, pin: u8, val: bool) {
            let raw_pin_num = (1u16 << pin as u16).to_be_bytes();

            self.bus.write(
                0x2a,
                &[
                if val { 0x09 } else { 0x0A },
                raw_pin_num[0],
                raw_pin_num[1],
                ],
                ).unwrap();
        }


        pub fn set_sens(&mut self, sens: u8) {
            self.sens = sens;
            self.analog_write(0, sens as u16);
        }

        pub fn read(&mut self) -> [u16; NUM_SENSORS] {
            const START: u8 = (8 - NUM_SENSORS as u8) / 2 + 1;
            const END: u8 = START + NUM_SENSORS as u8;
            debug_assert_eq!(END - START, NUM_SENSORS as u8);
            let vals: [u16; NUM_SENSORS] = unsafe { from_iter((START..END)
                                                              .into_iter()
                                                              .zip(self.correction)
                                                              .map(|(p, c)| ((self.analog_read(p) as i32) + c as i32) as u16))
                                                        .unwrap_unchecked() };
            vals
        }

        pub fn calibrate(&mut self, delay: &mut impl DelayMs<u16>) {
            const NUM_SAMPLES: usize = 20;

            let vals = self.read();
            rprintln!("Before calibration: corr={:?} vals={:?}", self.correction, vals);

            let mut samples = [[0_u16; NUM_SAMPLES]; NUM_SENSORS];
            for i in 0..NUM_SAMPLES {
                let vals = self.read();
                vals
                    .iter()
                    .zip(samples
                         .iter_mut())
                    .for_each(|(&val, s)| s[i] = val);
                delay.delay_ms(25);
            }
            let vals_averaged = samples
                .map(|vals| vals
                             .into_iter()
                             .map_into::<u32>()
                             .sum::<u32>() / vals.len() as u32);
            let mean = vals_averaged
                        .into_iter()
                        .sum::<u32>() / vals_averaged.len() as u32;
            self.correction = vals_averaged.map(|x| mean as i32 - x as i32);

            let vals = self.read();
            rprintln!("After calibration: corr={:?} vals{:?} mean={}", self.correction, vals, mean);
        }
    }

    #[shared]
    struct Shared {
        platform_stepper: Stepper<PB5<Output<PushPull>>, PA8<Output<PushPull>>>,
        platform_limit: PC7<Input<PullUp>>,
        enable_pin: PA9<Output<PushPull>>,
        serial_tx: serial::Tx<USART2>,
        serial_rx: serial::Rx<USART2>,
    }

    #[local]
    struct Local<'a> {
        speed: HertzU32,
        direction: StepperDireciton,
        line_sensor: LineSensor<I2cType>
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();

        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(84.mhz()).freeze();
        let mut syscfg = ctx.device.SYSCFG.constrain();

        let (gpioa, gpiob, gpioc) = (ctx.device.GPIOA.split(), ctx.device.GPIOB.split(), ctx.device.GPIOC.split());

        let mut delay = Delay::new(ctx.core.SYST, &clocks);

        let scl = gpiob.pb8.into_alternate_open_drain();
        let sda = gpiob.pb9.into_alternate_open_drain();
        let i2c = I2c::new(ctx.device.I2C1, (scl, sda), 400_000_u32, &clocks);

        let line_sensor = {
            let mut s = LineSensor::new(i2c);
            s.calibrate(&mut delay);
            s
        };

        let mut en = gpioa.pa9.into_push_pull_output();
        en.set_high();

        let platform_stepper = {
            let (step, dir) = (gpiob.pb5.into_push_pull_output(), gpioa.pa8.into_push_pull_output());
            let mut stepper = Stepper::new(step, dir, || platform::spawn().unwrap());
            stepper.set_direciton(StepperDireciton::CounterClockwise);
            stepper
        };

        let mut platform_limit = gpioc.pc7.into_pull_up_input();
        platform_limit.make_interrupt_source(&mut syscfg);
        platform_limit.enable_interrupt(&mut ctx.device.EXTI);
        platform_limit.trigger_on_edge(&mut ctx.device.EXTI, gpio::Edge::Falling);

        let mono = Timer::new(ctx.device.TIM2, &clocks).monotonic();

        let rx = gpioa.pa3.into_alternate();
        let tx = gpioa.pa2.into_alternate();
        let mut serial = Serial::<_, _, u8>::new(
            ctx.device.USART2,
            (tx, rx),
            Config::default().baudrate(1_000_000_u32.bps()),
            &clocks,
            )
            .unwrap();
	    serial.listen(Event::Rxne);

	    let (serial_tx, serial_rx) = serial.split();

        line::spawn().ok();

        (
            Shared {
                platform_stepper,
                platform_limit,
                enable_pin: en,
                serial_tx, serial_rx,
            },
            Local {
                speed: 400_u32.Hz(),
                direction: StepperDireciton::Clockwise,
                line_sensor
            },
            init::Monotonics(mono),
        )
    }

    #[task(local = [line_sensor], shared = [serial_tx])]
    fn line(mut cx: line::Context) {
        let vals: [_; NUM_SENSORS] = cx.local.line_sensor.read();

        let derivative: [i32; NUM_SENSORS - 1] = unsafe { from_iter(vals
                                                                    .into_iter()
                                                                    .tuple_windows()
                                                                    .map(|(a, b)| (b as i32) - (a as i32)))
                                                              .unwrap_unchecked() };

        let flip_idx = derivative
                        .into_iter()
                        .tuple_windows()
                        .map(|(a, b)| (a > 0) != (b > 0))
                        .enumerate()
                        .find(|(_, x)| *x)
                        .and_then(|(i, _)| Some(i));

        if let Some(flip_idx) = flip_idx {
            let left = {
                derivative[0..=flip_idx]
                    .into_iter()
                    .map(|x| x.abs())
                    .enumerate()
                    .filter(|(_, x)| *x > EDGE_THRESHOLD as i32)
                    .max_by_key(|(_, x)| *x)
                    .and_then(|(i, _)| Some(i))
            };
            let right = {
                derivative[flip_idx+1..]
                    .into_iter()
                    .map(|x| x.abs())
                    .enumerate()
                    .filter(|(_, x)| *x > EDGE_THRESHOLD as i32)
                    .max_by_key(|(_, x)| *x)
                    .and_then(|(i, _)| Some(i + flip_idx + 1))
            };
            // TODO: convert derivative index to mm
            if let Some(left) = left && let Some(right) = right {
                rprintln!("Line: {} {}", left, right);
            }
        }

        cx.shared.serial_tx.lock(|tx| {
            for v in derivative {
                let mut s = [0_u8; 11];
                tx.bwrite_all(v.numtoa(10, &mut s)).unwrap();
                block!(tx.write(' ' as u8)).unwrap();
            }
            block!(tx.write('\n' as u8)).unwrap();
        });

        line::spawn_after(100_u32.micros()).ok();
    }

    #[task(shared = [platform_stepper], priority = 15)]
    fn platform(mut cx: platform::Context) {
        cx.shared.platform_stepper.lock(|stepper| {
            let next_delay = stepper.update();
            if let Some(next_delay) = next_delay {
                platform::spawn_after(next_delay).ok();
            }
        });
    }

    #[task(binds = USART2, shared = [serial_rx], priority = 10)]
    fn uart_rx(mut cx: uart_rx::Context) {
        cx.shared.serial_rx.lock(|rx| {
            match rx.read() {
                Ok(byte) => {
                    rprintln!("Recv: {}", byte);
                },
                Err(e) => {
                    rprintln!("Err: {:?}", e);
                }
            }
        });
    }
}

