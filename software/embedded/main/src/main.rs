#![no_main]
#![no_std]

use panic_probe as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true, dispatchers = [EXTI1, EXTI2, EXTI3])]
mod app {
    use embedded_hal::digital::v2::OutputPin;
    use rtt_target::{rtt_init_print, rprintln};

    use stm32f4xx_hal::{
        prelude::*, pac, pac::USART2,
        timer, timer::{monotonic::MonoTimer, Timer},
        gpio, gpio::{
            gpioa::{PA10, PA9, PA8}, gpiob::{PB3, PB10, PB4, PB5},
            Output, PushPull, gpioc::PC4, Input, PullUp
        },
        serial, serial::{config::Config, Event, Serial}, pwm::PwmChannel,
    };
    use fugit::{RateExtU32, HertzU32};

    use stepper::{Stepper, StepperDireciton};

    use interfacing::{Interfacing, commands::{Command, SetSpeedParams}, CommandId};

    // TODO: kinda dirty but gonna go it for now
    const GRIPPER_OPEN_DUTIES: (u16, u16) = (1, 2);
    const GRIPPER_CLOSE_DUTIES: (u16, u16) = (2, 1);
    const PLATFORM_SPEED: u32 = 1500; // sps
    const PLATFORM_LOWER_TIME: u32 = 1000; // ms

    #[monotonic(binds = TIM2, default = true)]
    type MicrosecMono = MonoTimer<pac::TIM2, 1_000_000>;

    #[shared]
    struct Shared {
        left_stepper: Stepper<PA10<Output<PushPull>>, PB4<Output<PushPull>>>,
        right_stepper: Stepper<PB3<Output<PushPull>>, PB10<Output<PushPull>>>,
        platform_stepper: Stepper<PB5<Output<PushPull>>, PA8<Output<PushPull>>>,
        platform_limit: PC4<Input<PullUp>>,
        platform_lift_cmd: Option<CommandId>,
        enable_pin: PA9<Output<PushPull>>,
        servos: (PwmChannel<pac::TIM3, timer::C1>, PwmChannel<pac::TIM3, timer::C3>),
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
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        rtt_init_print!();

        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(84.mhz()).freeze();
        let mut syscfg = ctx.device.SYSCFG.constrain();

        let (gpioa, gpiob, gpioc) = (ctx.device.GPIOA.split(), ctx.device.GPIOB.split(), ctx.device.GPIOC.split());

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

        let platform_stepper = {
            let (step, dir) = (gpiob.pb5.into_push_pull_output(), gpioa.pa8.into_push_pull_output());
            let mut stepper = Stepper::new(step, dir, || platform::spawn().unwrap());
            stepper.set_direciton(StepperDireciton::CounterClockwise);
            stepper
        };

        let mut platform_limit = gpioc.pc4.into_pull_up_input();
        platform_limit.make_interrupt_source(&mut syscfg);
        platform_limit.enable_interrupt(&mut ctx.device.EXTI);
        platform_limit.trigger_on_edge(&mut ctx.device.EXTI, gpio::Edge::Falling);

        let servos = {
            let channels = (gpioc.pc6.into_alternate(), gpioc.pc8.into_alternate());
            let (mut ch1, mut ch3) = Timer::new(ctx.device.TIM3, &clocks).pwm(channels, 40u32.hz());
            ch1.enable();
            ch3.enable();
            (ch1, ch3)
        };

        let mono = Timer::new(ctx.device.TIM2, &clocks).monotonic();

        let rx = gpioa.pa3.into_alternate();
        let tx = gpioa.pa2.into_alternate();
        let mut serial = Serial::new(
            ctx.device.USART2,
            (tx, rx),
            Config::default().baudrate(interfacing::BAUD_RATE.bps()),
            &clocks,
            )
            .unwrap();
	    serial.listen(Event::Rxne);

	    let (serial_tx, serial_rx) = serial.split();

        //change_speed::spawn().ok();
        send_message::spawn().unwrap();

        (
            Shared {
                left_stepper,
                right_stepper,
                platform_stepper,
                platform_limit,
                platform_lift_cmd: None,
                enable_pin: en,
                servos,
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

    #[task(shared = [platform_stepper], priority = 15)]
    fn platform(mut cx: platform::Context) {
        cx.shared.platform_stepper.lock(|stepper| {
            let next_delay = stepper.update();
            if let Some(next_delay) = next_delay {
                platform::spawn_after(next_delay).ok();
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

    #[task(shared = [left_stepper, right_stepper, enable_pin, interfacing])]
    fn stop_cmd(mut cx: stop_cmd::Context, id: CommandId) {
        (cx.shared.left_stepper, cx.shared.right_stepper, cx.shared.enable_pin).lock(|left, right, en| {
            left.stop();
            right.stop();
            en.set_high();
        });

        cx.shared.interfacing.lock(|interfacing| {
            interfacing.finish_executing(id).unwrap();
        });
    }

    #[task(shared = [left_stepper, right_stepper, enable_pin, interfacing])]
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

        (cx.shared.left_stepper, cx.shared.right_stepper, cx.shared.enable_pin).lock(|left_stepper, right_stepper, en| {
            let SetSpeedParams{left, right} = params; 
            en.set_low();
            set_speed(left_stepper, &left);
            set_speed(right_stepper, &right);
        });

        cx.shared.interfacing.lock(|interfacing| {
            interfacing.finish_executing(id).unwrap();
        });
    }

    #[task(shared = [platform_stepper, platform_limit, enable_pin, platform_lift_cmd])]
    fn lift_platform_cmd(mut cx: lift_platform_cmd::Context, id: CommandId) {
        (cx.shared.platform_stepper, cx.shared.enable_pin).lock(|stepper, en| {
            en.set_low();
            stepper.set_direciton(StepperDireciton::Clockwise);
            stepper.set_speed(PLATFORM_SPEED.Hz());
        });

        cx.shared.platform_lift_cmd.lock(|platform_lift_cmd| {
            if platform_lift_cmd.is_some() {
                // TODO: handle
            }
            *platform_lift_cmd = Some(id);
        });
    }

    #[task(shared = [platform_stepper, platform_limit, enable_pin])]
    fn lower_platform_cmd(cx: lower_platform_cmd::Context, id: CommandId) {
        (cx.shared.platform_stepper, cx.shared.enable_pin).lock(|stepper, en| {
            en.set_low();
            stepper.set_direciton(StepperDireciton::CounterClockwise);
            stepper.set_speed(PLATFORM_SPEED.Hz());
        });
        stop_platform_lower::spawn_after(PLATFORM_LOWER_TIME.millis(), id).unwrap();
    }

    #[task(shared = [platform_stepper, interfacing])]
    fn stop_platform_lower(mut cx: stop_platform_lower::Context, id: CommandId) {
        cx.shared.platform_stepper.lock(|stepper| {
            stepper.stop();
        });

        cx.shared.interfacing.lock(|interfacing| {
            interfacing.finish_executing(id).unwrap();
        });
    }

    #[task(binds = EXTI4, shared = [platform_stepper, platform_limit, interfacing, platform_lift_cmd])]
    fn stop_platform(mut cx: stop_platform::Context) {
        (cx.shared.platform_stepper, cx.shared.platform_limit).lock(|stepper, limit| {
            limit.clear_interrupt_pending_bit();
            stepper.stop();
        });

        cx.shared.platform_lift_cmd.lock(|platform_lift_cmd| {
            if let Some(id) = platform_lift_cmd.take() {
                cx.shared.interfacing.lock(|interfacing| {
                    interfacing.finish_executing(id).unwrap();
                });
            }
        });
    }

    #[task(shared = [servos, interfacing])]
    fn open_gripper_cmd(mut cx: open_gripper_cmd::Context, id: CommandId) {
        cx.shared.servos.lock(|servos| {
            let (left, right) = servos;
            let (left_duty, right_duty) = GRIPPER_OPEN_DUTIES;
            left.set_duty(left_duty);
            right.set_duty(right_duty);
        });

        cx.shared.interfacing.lock(|interfacing| {
            interfacing.finish_executing(id).unwrap();
        });
    }

    #[task(shared = [servos, interfacing])]
    fn close_gripper_cmd(mut cx: close_gripper_cmd::Context, id: CommandId) {
        cx.shared.servos.lock(|servos| {
            // TODO: duplication
            let (left, right) = servos;
            let (left_duty, right_duty) = GRIPPER_CLOSE_DUTIES;
            left.set_duty(left_duty);
            right.set_duty(right_duty);
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
                Command::LiftGripper => lift_platform_cmd::spawn(id).unwrap(),
                Command::LowerGripper => lower_platform_cmd::spawn(id).unwrap(),
                Command::OpenGripper => open_gripper_cmd::spawn(id).unwrap(),
                Command::CloseGripper => close_gripper_cmd::spawn(id).unwrap(),
            }
        });
    }

    #[task(binds = USART2, shared = [serial_rx, interfacing], priority = 10)]
    fn uart_rx(cx: uart_rx::Context) {
        (cx.shared.serial_rx, cx.shared.interfacing).lock(|rx, interfacing| {
            match rx.read() {
                Ok(byte) => {
                    let res = interfacing.handle_received_byte(byte);
                    if let Err(e) = res {
                        rprintln!("Error while receiving a message: {:?}", e);
                        return;
                    }

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

