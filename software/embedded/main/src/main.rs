#![no_main]
#![no_std]

use panic_probe as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [EXTI1, EXTI2, EXTI3])]
mod app {
    use embedded_hal::digital::v2::OutputPin;
    use rtt_target::{rtt_init_print, rprintln, rprint};

    use stm32f4xx_hal::{
        prelude::*, pac, pac::USART2,
        block,
        timer, timer::{monotonic::MonoTimer, Timer},
        gpio, gpio::{
            gpioa::{PA10, PA9, PA8}, gpiob::{PB3, PB10, PB4, PB5},
            Output, PushPull, gpioc::PC7, Input, PullUp,
        },
        i2c::I2c,
        serial, serial::{config::Config, Event, Serial}, pwm::PwmChannel,
    };
    use fugit::{RateExtU32, HertzU32, Duration};

    use gpio_expander::{prelude::*, GpioExpander, Pin};

    use numtoa::NumToA;

    use heapless::Vec;

    use itertools::Itertools;

    use stepper::{Stepper, StepperDireciton};

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimer<pac::TIM2, 1_000_000>;

    #[shared]
    struct Shared {
        platform_stepper: Stepper<PB5<Output<PushPull>>, PA8<Output<PushPull>>>,
        platform_limit: PC7<Input<PullUp>>,
        enable_pin: PA9<Output<PushPull>>,
        serial_tx: serial::Tx<USART2>,
        serial_rx: serial::Rx<USART2>,
    }

    #[local]
    struct Local {
        speed: HertzU32,
        direction: StepperDireciton
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

        let mut expander = GpioExpander::new(i2c, None);
        let expander_pins = expander.pins();

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
            Config::default().baudrate(interfacing::BAUD_RATE.bps()),
            &clocks,
            )
            .unwrap();
	    serial.listen(Event::Rxne);

	    let (mut serial_tx, serial_rx) = serial.split();

        let pins: [Pin<_, _>; 8] = [
            expander_pins.p01.into_input().unwrap(),
            expander_pins.p02.into_input().unwrap(),
            expander_pins.p02.into_input().unwrap(),
            expander_pins.p02.into_input().unwrap(),
            expander_pins.p02.into_input().unwrap(),
            expander_pins.p02.into_input().unwrap(),
            expander_pins.p02.into_input().unwrap(),
            expander_pins.p02.into_input().unwrap(),
        ];

        let mut delay = stm32f4xx_hal::delay::Delay::new(ctx.core.SYST, &clocks);
        loop {
            let vals = {
                let mut vals = [0_u16; 8];
                for (p, v) in pins.iter().zip(vals.iter_mut()) {
                    *v = p.get_analog().unwrap();
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

            for v in vals {
                let mut s = [0_u8; 10];
                serial_tx.bwrite_all(v.numtoa(10, &mut s)).unwrap();
                block!(serial_tx.write(' ' as u8)).unwrap();
            }
            block!(serial_tx.write('\n' as u8)).unwrap();

            delay.delay_ms(100_u32);
        }

        //change_speed::spawn().ok();

        (
            Shared {
                platform_stepper,
                platform_limit,
                enable_pin: en,
                serial_tx, serial_rx,
            },
            Local {
                speed: 400_u32.Hz(),
                direction: StepperDireciton::Clockwise
            },
            init::Monotonics(mono),
        )
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

