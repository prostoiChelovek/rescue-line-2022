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
            gpioa::PA8, gpiob::PB10,
            Output, PushPull
        },
    };
    use fugit::{RateExtU32, HertzU32};

    use a49xx::A49xx;

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimer<pac::TIM2, 1_000_000>;

    #[shared]
    struct Shared {
        stepper: A49xx<PA8<Output<PushPull>>, PB10<Output<PushPull>>>
    }

    #[local]
    struct Local {
        speed: HertzU32
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();

        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(84.mhz()).freeze();

        let (gpioa, gpiob) = (ctx.device.GPIOA.split(), ctx.device.GPIOB.split());

        let (step, dir) = (gpioa.pa8.into_push_pull_output(), gpiob.pb10.into_push_pull_output());
        let stepper = A49xx::new(step, dir, || test::spawn().unwrap());

        let mono = Timer::new(ctx.device.TIM2, &clocks).monotonic();

        change_speed::spawn().ok();

        (
            Shared {
                stepper
            },
            Local {
                speed: 1500_u32.Hz()
            },
            init::Monotonics(mono),
        )
    }

    #[task(shared = [stepper], local = [speed])]
    fn change_speed(mut cx: change_speed::Context) {
        cx.shared.stepper.lock(|stepper| {
            let speed = cx.local.speed;
            *speed = *speed + 100_u32.Hz();
            if *speed >= 1500_u32.Hz::<1_u32, 1_u32>() { *speed = 100_u32.Hz() }

            rprintln!("{}", speed);
            stepper.set_speed(*speed);
        });

        change_speed::spawn_after(500_u32.millis()).ok();
    }

    #[task(shared = [stepper], priority = 15)]
    fn test(mut cx: test::Context) {
        cx.shared.stepper.lock(|stepper| {
            let next_delay = stepper.update();
            if let Some(next_delay) = next_delay {
                test::spawn_after(next_delay).ok();
            }
        });
    }
}

