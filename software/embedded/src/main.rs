#![no_main]
#![no_std]

use panic_rtt_target as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [USART1])]
mod app {
    use rtt_target::{rtt_init_print, rprintln};

    use stm32f4xx_hal::{
        prelude::*, pac,
        timer::{monotonic::MonoTimer, Timer},
    };

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimer<pac::TIM2, 1_000_000>;

    #[shared]
    struct Shared { }

    #[local]
    struct Local { }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();

        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(84.mhz()).freeze();

        let mono = Timer::new(ctx.device.TIM2, &clocks).monotonic();

        (
            Shared { },
            Local { },
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        rprintln!("Hello, world");
        loop { }
    }
}
