#![no_main]
#![no_std]
#![feature(let_chains)]

use panic_probe as _;

mod line_sensor;

macro_rules! wheel_alias {
    ($name:ident, $dir_1_pin:ident, $dir_2_pin:ident, $pwm_timer:ident, $pwm_chan:ident, $qei_pin_1:ident, $qei_pin_2:ident, $qei_tim:ident, $qei_af:literal) => {
        mod $name {
            use super::*;

            type _DriverT = TwoWirteDriver<PwmChannel<$pwm_timer, $pwm_chan>,
                                           $dir_1_pin<OutPP>>;

            mod _encoder {
                use super::*;

                type EncoderPinMode = Alternate<PushPull, $qei_af>;
                type QeiT = Qei<$qei_tim, ($qei_pin_1<EncoderPinMode>, $qei_pin_2<EncoderPinMode>)>;
                pub type Encoder = RotaryEncoder<QeiT>;
            }

            pub type WheelT = Wheel<_DriverT, _encoder::Encoder>;

            pub use _encoder::Encoder as EncoderT;
        }
    };
}

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [EXTI1, EXTI2, EXTI3])]
mod app {
    use core::fmt::Write;

    use embedded_hal::{digital::v2::OutputPin, blocking::{i2c, delay::DelayMs}};
    use motor::GetSpeed;
    use rtt_target::{rtt_init_print, rprintln, rprint};

    use stm32f4xx_hal::{
        prelude::*, pac, pac::{USART2, TIM1, TIM3, TIM5},
        block, delay::Delay,
        timer, timer::{monotonic::MonoTimer, Timer},
        gpio, gpio::{
            gpioa::{PA0, PA1, PA9, PA8},
            gpiob::{PB3, PB10, PB4, PB5, PB6},
            gpioc::{PC7, PC8},
            gpiob,
            Output, PushPull, Input, PullUp, Alternate
        },
        i2c::I2c,
        serial, serial::{config::Config, Event, Serial},
        pwm::{PwmChannel, C1, C2},
        qei::Qei
    };

    use fugit::{RateExtU32, HertzU32, Duration};
    use numtoa::NumToA;
    use array_init::from_iter;
    use itertools::Itertools;

    use pid::Pid;

    use stepper::{Stepper, StepperDireciton};
    use rotary_encoder::RotaryEncoder;
    use encoder::Update;
    use dc_motor::TwoWirteDriver;
    use wheel::Wheel;

    use crate::line_sensor::{LineSensor, NUM_SENSORS};

    const LINE_DEBUG: bool = false;
    const WHEELS_DEBUG: bool = true;

    const EDGE_THRESHOLD: u16 = 110;
    const WHEEL_MIN_DUTY: u8 = 100;
    const WHEEL_ENCODER_PPR: f32 = 48.0;
    const WHEEL_MAX_ROTARY_SPEED: f32 = 100.0;
    const WHEEL_RADIUS: f32 = 100.0;

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimer<pac::TIM2, 1_000_000>;

    type OutPP = Output<PushPull>;

    wheel_alias!(left_wheel, PB10, PB3, TIM1, C1, PB4, PB5, TIM3, 2_u8);
    wheel_alias!(right_wheel, PC7, PB6, TIM1, C2, PA0, PA1, TIM5, 2_u8);

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
        left: left_wheel::WheelT,
        right: right_wheel::WheelT,

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

        /*
        let mut en = gpioa.pa9.into_push_pull_output();
        en.set_high();

        let platform_stepper = {
            let (step, dir) = (gpiob.pb5.into_push_pull_output(), gpioa.pa8.into_push_pull_output());
            let mut stepper = Stepper::new(step, dir, || platform::spawn().unwrap());
            stepper.set_direciton(StepperDireciton::CounterClockwise);
            stepper
        };

        let mut platform_limit = gpioc.pc8.into_pull_up_input();
        platform_limit.make_interrupt_source(&mut syscfg);
        platform_limit.enable_interrupt(&mut ctx.device.EXTI);
        platform_limit.trigger_on_edge(&mut ctx.device.EXTI, gpio::Edge::Falling);
        */

        let mono = Timer::new(ctx.device.TIM2, &clocks).monotonic();

        let (left_wheel, right_wheel) = {
            let en_pins = (gpioa.pa8.into_alternate(), gpioa.pa9.into_alternate());
            let en_pwms = Timer::new(ctx.device.TIM1, &clocks).pwm(en_pins, 2.khz());
            let (left_en_pwm, right_en_pwm) = en_pwms;

            let speed_pid = Pid::new(0.25, 0.02, 1.0,
                               100.0, 100.0, 100.0,
                               100.0,
                               0.0);

            ({
                let in_1 = gpiob.pb10.into_push_pull_output();

                let motor = TwoWirteDriver::new(left_en_pwm, in_1, WHEEL_MIN_DUTY);

                let encoder_pins = (gpiob.pb4.into_alternate(), gpiob.pb5.into_alternate());
                let encoder_timer = ctx.device.TIM3;
                let qei = Qei::new(encoder_timer, encoder_pins);
                let encoder = RotaryEncoder::new(qei, WHEEL_ENCODER_PPR, true);

                Wheel::new(motor, encoder, speed_pid.clone(), WHEEL_MAX_ROTARY_SPEED, WHEEL_RADIUS)
            },
            {
                let in_1 = gpioc.pc7.into_push_pull_output();

                let motor = TwoWirteDriver::new(right_en_pwm, in_1, WHEEL_MIN_DUTY);

                let encoder_pins = (gpioa.pa0.into_alternate(), gpioa.pa1.into_alternate());
                let encoder_timer = ctx.device.TIM5;
                let qei = Qei::new(encoder_timer, encoder_pins);
                let encoder = RotaryEncoder::new(qei, WHEEL_ENCODER_PPR, true);

                Wheel::new(motor, encoder, speed_pid.clone(), WHEEL_MAX_ROTARY_SPEED, WHEEL_RADIUS)
            })
        };

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
        updater::spawn().ok();
        if WHEELS_DEBUG { speed_printer::spawn().ok(); }

        (
            Shared {
                left: left_wheel,
                right: right_wheel,
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
                if LINE_DEBUG { block!(tx.write(' ' as u8)).unwrap(); }
            }
            for v in vals {
                let mut s = [0_u8; 11];
                tx.bwrite_all(v.numtoa(10, &mut s)).unwrap();
                if LINE_DEBUG { block!(tx.write(' ' as u8)).unwrap(); }
            }
            if LINE_DEBUG { block!(tx.write('\n' as u8)).unwrap(); }
        });

        line::spawn_after(100_u32.micros()).ok();
    }

    #[task(shared = [left, right, serial_tx])]
    fn speed_printer(cx: speed_printer::Context) {
        (cx.shared.left, cx.shared.right,
         cx.shared.serial_tx).lock(|left, right, serial_tx| {
            writeln!(serial_tx, "{} {} {} {}", left.get_target_speed(), left.get_speed(),
                                               right.get_target_speed(), right.get_speed()).ok();
        });
        speed_printer::spawn_after(100_u32.millis()).ok();
    }


    #[task(shared = [left, right])]
    fn updater(mut cx: updater::Context) {
        const TIME_DELTA_SECONDS: f32 = 0.025;

        cx.shared.left.lock(|left| {
            left.update(TIME_DELTA_SECONDS);
        });
        cx.shared.right.lock(|right| {
            right.update(TIME_DELTA_SECONDS);
        });

        updater::spawn_after(25.millis()).ok();
    }

    /*
    #[task(shared = [platform_stepper], priority = 15)]
    fn platform(mut cx: platform::Context) {
        cx.shared.platform_stepper.lock(|stepper| {
            let next_delay = stepper.update();
            if let Some(next_delay) = next_delay {
                platform::spawn_after(next_delay).ok();
            }
        });
    }
    */

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

