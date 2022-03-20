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
    use fugit::RateExtU32;

    use a49xx::A49xx;

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimer<pac::TIM2, 1_000_000>;

    #[shared]
    struct Shared { }

    #[local]
    struct Local {
        stepper: A49xx<PA8<Output<PushPull>>, PB10<Output<PushPull>>>
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();

        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(84.mhz()).freeze();

        let (gpioa, gpiob) = (ctx.device.GPIOA.split(), ctx.device.GPIOB.split());

        let (step, dir) = (gpioa.pa8.into_push_pull_output(), gpiob.pb10.into_push_pull_output());
        let mut stepper = A49xx::new(step, dir);
        stepper.set_speed(1500_u32.Hz());

        let mono = Timer::new(ctx.device.TIM2, &clocks).monotonic();

        test::spawn().ok();

        (
            Shared { },
            Local {
                stepper
            },
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        rprintln!("Hello, world");

        loop { }
    }

    #[task(local = [stepper], priority = 15)]
    fn test(cx: test::Context) {
        let stepper = cx.local.stepper;
        let next_delay = stepper.update();
        if let Some(next_delay) = next_delay {
            test::spawn_after(next_delay).ok();
        }
    }
}

