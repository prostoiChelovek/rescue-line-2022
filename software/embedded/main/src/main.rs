#![no_main]
#![no_std]

use panic_probe as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [EXTI1, EXTI2, EXTI3])]
mod app {
    use embedded_hal::{digital::v2::OutputPin, blocking::i2c};
    use rtt_target::{rtt_init_print, rprintln, rprint};

    use stm32f4xx_hal::{
        prelude::*, pac, pac::USART2,
        block,
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

    use itertools::Itertools;

    use stepper::{Stepper, StepperDireciton};

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimer<pac::TIM2, 1_000_000>;

    type I2cType = I2c<
        pac::I2C1,
        (gpiob::PB8<gpio::Alternate<gpio::OpenDrain, 4>>,
         gpiob::PB9<gpio::Alternate<gpio::OpenDrain, 4>>),
         >;

    pub struct LineSensor<BUS: i2c::Write + i2c::WriteRead> {
        bus: BUS
    }

    impl<BUS, E> LineSensor<BUS>
    where
        BUS: i2c::Write<Error = E> + i2c::WriteRead<Error = E>,
        E: core::fmt::Debug
    {
        fn map_pin(pin: u8) -> u8 {
            // who the fuck came up with this shit numbering scheme
            [4, 5, 6, 8, 7, 3, 2, 1][pin as usize - 1]
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

        let scl = gpiob.pb8.into_alternate_open_drain();
        let sda = gpiob.pb9.into_alternate_open_drain();
        let i2c = I2c::new(ctx.device.I2C1, (scl, sda), 400_000_u32, &clocks);

        let line_sensor = {
            let mut s = LineSensor { bus: i2c };
            for pin in 1..=8 {
                s.into_input(pin);
            }
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
        let vals = {
            let mut vals = [0_u16; 8];
            for (p, v) in (1..=8).into_iter().zip(vals.iter_mut()) {
                *v = cx.local.line_sensor.analog_read(p);
            }
            vals
        };

        let line = {
            let d = vals
                .into_iter()
                .tuple_windows()
                .map(|(a, b)| ((a as i32) - (b as i32)).abs() as u16);
            let mean = d.clone().sum::<u16>() / (vals.len() - (vals.len() % 2)) as u16;

            d.clone().for_each(|x| rprint!("{} ", x));
            rprintln!("{}", mean);

            let mut r = [false; 7];
            d
                .map(|x| x > mean)
                .zip(r.iter_mut())
                .for_each(|(x, v)| *v = x);
            r
        };

        for x in line {
            rprint!("{} ", x);
        }
        rprintln!();


        cx.shared.serial_tx.lock(|tx| {
            for v in vals {
                let mut s = [0_u8; 10];
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

