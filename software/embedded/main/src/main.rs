#![no_main]
#![no_std]

use panic_probe as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [EXTI1, EXTI2])]
mod app {
    use rtt_target::{rtt_init_print, rprintln};

    use stm32f4xx_hal::{
        prelude::*, pac,
        timer::{monotonic::MonoTimer, Timer},
        gpio::{
            gpioa::{PA8, PA10}, gpiob::{PB3, PB10, PB5, PB4},
            Output, PushPull
        },
    };
    use fugit::{RateExtU32, HertzU32};

    use stepper::{Stepper, StepperDireciton};

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimer<pac::TIM2, 1_000_000>;

    #[shared]
    struct Shared {
        right_stepper: Stepper<PB10<Output<PushPull>>, PA8<Output<PushPull>>>,
        left_stepper: Stepper<PA10<Output<PushPull>>, PB3<Output<PushPull>>>
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

        let right_stepper = {
            let (step, dir) = (gpiob.pb10.into_push_pull_output(), gpioa.pa8.into_push_pull_output());
            let mut stepper = Stepper::new(step, dir, || test::spawn().unwrap());
            stepper.set_direciton(StepperDireciton::CounterClockwise);
            stepper.set_speed(5_u32.Hz());
            stepper
        };

        let left_stepper = {
            let (step, dir) = (gpioa.pa10.into_push_pull_output(), gpiob.pb3.into_push_pull_output());
            let mut stepper = Stepper::new(step, dir, || right::spawn().unwrap());
            stepper.set_direciton(StepperDireciton::Clockwise);
            stepper.set_speed(5_u32.Hz());
            stepper
        };



        let mono = Timer::new(ctx.device.TIM2, &clocks).monotonic();

        //change_speed::spawn().ok();

        (
            Shared {
                left_stepper,
                right_stepper
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
}

