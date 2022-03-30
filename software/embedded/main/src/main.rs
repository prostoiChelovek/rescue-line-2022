#![no_main]
#![no_std]

use panic_probe as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [EXTI1, EXTI2, EXTI3])]
mod app {
    use embedded_hal::digital::v2::OutputPin;
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

    use interfacing::{Interfacing, commands::{Command, SetSpeedParams}, CommandId};

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
        en.set_low();
        
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
        send_message::spawn().unwrap();

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

    #[task(shared = [interfacing, serial_tx], priority = 10)]
    fn send_message(cx: send_message::Context) {
        (cx.shared.serial_tx, cx.shared.interfacing).lock(|tx, interfacing| {
            if let Some(msg) = interfacing.get_message_to_send() {
                tx.bwrite_all(&msg[..]).unwrap();
            }
        });
        send_message::spawn_after(10_u32.millis()).unwrap();
    }

    #[task(shared = [left_stepper, right_stepper, interfacing])]
    fn stop_cmd(mut cx: stop_cmd::Context, id: CommandId) {
        (cx.shared.left_stepper, cx.shared.right_stepper).lock(|left, right| {
            left.stop();
            right.stop();
        });

        cx.shared.interfacing.lock(|interfacing| {
            interfacing.finish_executing(id).unwrap();
        });
    }

    #[task(shared = [left_stepper, right_stepper, interfacing])]
    fn set_speed_cmd(mut cx: set_speed_cmd::Context, id: CommandId, params: SetSpeedParams) {
        fn set_speed<S: OutputPin, D: OutputPin>(stepper: &mut Stepper<S, D>, speed: &i32) {
            let stepper_speed = (speed.abs() as u32).Hz();
            if *speed < 0 {
                stepper.set_direciton(StepperDireciton::CounterClockwise);
                stepper.set_speed(stepper_speed);
            } else if *speed == 0 {
                stepper.stop();
            } else if *speed > 0 {
                stepper.set_direciton(StepperDireciton::Clockwise);
                stepper.set_speed(stepper_speed);
            }
        }

        (cx.shared.left_stepper, cx.shared.right_stepper).lock(|left_stepper, right_stepper| {
            let SetSpeedParams{left, right} = params; 
            set_speed(left_stepper, &left);
            set_speed(right_stepper, &right);
        });

        cx.shared.interfacing.lock(|interfacing| {
            interfacing.finish_executing(id).unwrap();
        });
    }

    #[task(shared = [interfacing])]
    fn handle_command(mut cx: handle_command::Context, id: CommandId) {
        cx.shared.interfacing.lock(|interfacing| {
            let cmd = interfacing.get_command(id);
            rprintln!("cmd: {:?}", cmd);

            match cmd {
                Command::Stop => stop_cmd::spawn(id).unwrap(),
                Command::SetSpeed(params) => set_speed_cmd::spawn(id, params).unwrap(),
                _ => {}
            }
        });
    }

    #[task(binds = USART2, shared = [serial_rx, interfacing], priority = 10)]
    fn uart_rx(cx: uart_rx::Context) {
        (cx.shared.serial_rx, cx.shared.interfacing).lock(|rx, interfacing| {
            match rx.read() {
                Ok(byte) => {
                    interfacing.handle_received_byte(byte).unwrap();
                    if let Some(cmd) = interfacing.get_command_to_execute() {
                        handle_command::spawn(cmd).unwrap();
                    }
                },
                Err(_e) => {
                    // TODO
                }
            }
        });
    }
}

