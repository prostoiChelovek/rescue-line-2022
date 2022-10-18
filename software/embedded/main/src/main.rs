#![no_main]
#![no_std]
#![feature(let_chains)]

use panic_probe as _;

mod line_sensor;

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

    use array_init::from_iter;

    use itertools::Itertools;

    use stepper::{Stepper, StepperDireciton};

    use crate::line_sensor::{LineSensor, NUM_SENSORS};

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

        let line = {
            let derivative_th = derivative
                .into_iter()
                .enumerate()
                .filter(|(_, x)| x.abs() > EDGE_THRESHOLD as i32);
            let first_edge = derivative_th.clone().max_by_key(|(_, x)| *x);
            (
                first_edge.and_then(|(i, _)| Some(i)),
                first_edge.and_then(|(_, first_val)|
                                    derivative_th
                                        .clone()
                                        .filter(|(_, x)| (*x > 0) != (first_val > 0))
                                        .min_by_key(|(_, x)| *x)
                                        .and_then(|(i, _)| Some(i)))
            )
        };

        // TODO: convert derivative index to mm
        if line.0.is_some() || line.1.is_some() {
            rprintln!("Line: {:?}", line);
        }

        cx.shared.serial_tx.lock(|tx| {
            for v in derivative {
                let mut s = [0_u8; 11];
                tx.bwrite_all(v.numtoa(10, &mut s)).unwrap();
                block!(tx.write(' ' as u8)).unwrap();
            }
            for v in vals {
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

