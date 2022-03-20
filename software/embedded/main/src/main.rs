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

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimer<pac::TIM2, 1_000_000>;

    #[shared]
    struct Shared { }

    #[local]
    struct Local {
        step: PA8<Output<PushPull>>,
        dir: PB10<Output<PushPull>>,
        step_pulse: bool
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();

        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(84.mhz()).freeze();

        let (gpioa, gpiob) = (ctx.device.GPIOA.split(), ctx.device.GPIOB.split());

        let (step, dir) = (gpioa.pa8.into_push_pull_output(), gpiob.pb10.into_push_pull_output());

        let mono = Timer::new(ctx.device.TIM2, &clocks).monotonic();

        test::spawn().ok();

        (
            Shared { },
            Local {
                step, dir,
                step_pulse: false
            },
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        rprintln!("Hello, world");

        loop { }
    }

    #[task(local = [step, dir, step_pulse], priority = 15)]
    fn test(cx: test::Context) {
        let (step, _dir) = (cx.local.step, cx.local.dir);
        let step_pulse = cx.local.step_pulse;

        step.toggle();
        if !*step_pulse {
            test::spawn_after(2.millis()).ok();
            *step_pulse = true;
        } else {
            test::spawn_after(2.micros()).ok();
            *step_pulse = false;
        }

    }
}

