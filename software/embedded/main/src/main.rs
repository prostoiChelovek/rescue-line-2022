#![no_main]
#![no_std]

use panic_probe as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [EXTI1, EXTI2])]
mod app {
    use rtt_target::{rtt_init_print, rprintln};

    use stm32f4xx_hal::{
        prelude::*, pac, pac::{USART2, NVIC, Interrupt},
        timer::{monotonic::MonoTimer, Timer},
        gpio::{
            gpioa::PA10, gpiob::{PB3, PB10, PB4},
            Output, PushPull
        },
        serial, serial::{config::Config, Event, Serial},
    };
    use fugit::{RateExtU32, HertzU32};

    use stepper::{Stepper, StepperDireciton};

    use interfacing::Interfacing;

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimer<pac::TIM2, 1_000_000>;

    #[shared]
    struct Shared {
        left_stepper: Stepper<PA10<Output<PushPull>>, PB4<Output<PushPull>>>,
        right_stepper: Stepper<PB3<Output<PushPull>>, PB10<Output<PushPull>>>,
        interfacing: Interfacing,
        serial_tx: serial::Tx<USART2>,
        serial_rx: serial::Rx<USART2>,
    }

    #[local]
    struct Local {
        speed: HertzU32,
        direction: StepperDireciton
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();

        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(84.mhz()).freeze();

        let (gpioa, gpiob) = (ctx.device.GPIOA.split(), ctx.device.GPIOB.split());

        let mut en = gpioa.pa9.into_push_pull_output();
        en.set_high();
        
        let left_stepper = {
            let (step, dir) = (gpioa.pa10.into_push_pull_output(), gpiob.pb4.into_push_pull_output());
            let mut stepper = Stepper::new(step, dir, || test::spawn().unwrap());
            stepper.set_direciton(StepperDireciton::CounterClockwise);
            //stepper.set_speed(400_u32.Hz());
            stepper
        };

        let right_stepper = {
            let (step, dir) = (gpiob.pb3.into_push_pull_output(), gpiob.pb10.into_push_pull_output());
            let mut stepper = Stepper::new(step, dir, || right::spawn().unwrap());
            stepper.set_direciton(StepperDireciton::CounterClockwise);
            //stepper.set_speed(100_u32.Hz());
            stepper
        };

        let mono = Timer::new(ctx.device.TIM2, &clocks).monotonic();

        let tx = gpioa.pa2.into_alternate();
        let rx = gpioa.pa3.into_alternate();
        let mut serial = Serial::new(
            ctx.device.USART2,
            (tx, rx),
            Config::default().baudrate(interfacing::BAUD_RATE.bps()),
            &clocks,
            )
            .unwrap();
	    serial.listen(Event::Rxne);
	
	    // Enable interrupt
	    NVIC::unpend(Interrupt::USART2);
	    unsafe {
	        NVIC::unmask(Interrupt::USART2);
	    }
	
	    let (serial_tx, serial_rx) = serial.split();

        //change_speed::spawn().ok();

        (
            Shared {
                left_stepper,
                right_stepper,
                interfacing: Interfacing::new(),
                serial_tx, serial_rx,
            },
            Local {
                speed: 400_u32.Hz(),
                direction: StepperDireciton::Clockwise
            },
            init::Monotonics(mono),
        )
    }

    #[task(shared = [left_stepper], local = [speed, direction])]
    fn change_speed(mut cx: change_speed::Context) {
        cx.shared.left_stepper.lock(|stepper| {
            let speed = cx.local.speed;
            let direction = cx.local.direction;
            *speed = *speed + 100_u32.Hz();
            if *speed >= 1500_u32.Hz::<1_u32, 1_u32>() {
                *speed = 100_u32.Hz();
                *direction = match *direction {
                    StepperDireciton::Clockwise => { StepperDireciton::CounterClockwise },
                    StepperDireciton::CounterClockwise => { StepperDireciton::Clockwise } 
                };
            }

            rprintln!("{}", speed);
            stepper.set_speed(*speed);
            stepper.set_direciton(direction.clone());
        });

        change_speed::spawn_after(500_u32.millis()).ok();
    }

    #[task(shared = [left_stepper], priority = 15)]
    fn test(mut cx: test::Context) {
        cx.shared.left_stepper.lock(|stepper| {
            let next_delay = stepper.update();
            if let Some(next_delay) = next_delay {
                test::spawn_after(next_delay).ok();
            }
        });
    }

    #[task(shared = [right_stepper], priority = 15)]
    fn right(mut cx: right::Context) {
        cx.shared.right_stepper.lock(|stepper| {
            let next_delay = stepper.update();
            if let Some(next_delay) = next_delay {
                right::spawn_after(next_delay).ok();
            }
        });
    }

    #[task(binds = USART2, shared = [serial_rx, serial_tx)]
    fn uart_rx(cx: uart_rx::Context) {
        cx.shared.serial_rx.lock(|rx| {
            match rx.read() {
                Ok(_byte) => {

                },
                Err(_e) => {
                    // TODO
                }
            }
        });
    }
}

