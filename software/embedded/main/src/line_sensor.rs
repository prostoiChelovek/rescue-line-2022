use rtt_target::rprintln;
use embedded_hal::blocking::{i2c, delay::DelayMs};

use array_init::from_iter;
use itertools::Itertools;

pub const NUM_SENSORS: usize = 6;

pub struct LineSensor<BUS: i2c::Write + i2c::WriteRead> {
    bus: BUS,
    sens: u8,
    correction: [i32; NUM_SENSORS]
}

impl<BUS, E> LineSensor<BUS>
where
    BUS: i2c::Write<Error = E> + i2c::WriteRead<Error = E>,
    E: core::fmt::Debug
{
    pub fn new(bus: BUS) -> Self {
        let mut res = Self {
            bus,
            sens: 0,
            correction: [0; NUM_SENSORS]

        };

        // TODO: not the best practive to do this in constructor
        res.into_output(0); // sens
        res.into_output(9); // IR leds
        for pin in 1..=8 {
            res.into_input(pin);
        }
        res.digital_write(9, true); // enable the IR leds

        res
    }

    fn map_pin(pin: u8) -> u8 {
        // who the fuck came up with this shit numbering scheme
        [0, 4, 5, 6, 8, 7, 3, 2, 1, 9][pin as usize]
    }

    pub fn into_input(&mut self, pin: u8) {
        let pin = Self::map_pin(pin);

        let raw_pin_num = (1u16 << pin as u16).to_be_bytes();
        self.bus.write(
            0x2a,
            &[
            0x04,
            raw_pin_num[0],
            raw_pin_num[1],
            ],
            ).unwrap();
    }

    pub fn into_output(&mut self, pin: u8) {
        let pin = Self::map_pin(pin);

        let raw_pin_num = (1u16 << pin as u16).to_be_bytes();
        self.bus.write(
            0x2a,
            &[
            0x07,
            raw_pin_num[0],
            raw_pin_num[1],
            ],
            ).unwrap();
    }

    pub fn analog_read(&mut self, pin: u8) -> u16 {
        let pin = Self::map_pin(pin);

        let mut inner = || {
            let write_buf = [0x0C, pin];
            let mut read_buf = [0u8; 2];

            self.bus
                .write_read(0x2a, &write_buf, &mut read_buf)
                .unwrap();

            u16::from_be_bytes(read_buf)
        };

        // TODO: a hack to get around a bug that causes the sensor get output a value for the
        //       previously requested pin
        inner();
        inner()
    }

    pub fn analog_write(&mut self, pin: u8, val: u16) {
        let raw_value: [u8; 2] = val.to_be_bytes();

        self.bus.write(
            0x2a,
            &[
            0x0B,
            pin,
            raw_value[0],
            raw_value[1],
            ],
            ).unwrap();
    }

    pub fn digital_write(&mut self, pin: u8, val: bool) {
        let raw_pin_num = (1u16 << pin as u16).to_be_bytes();

        self.bus.write(
            0x2a,
            &[
            if val { 0x09 } else { 0x0A },
            raw_pin_num[0],
            raw_pin_num[1],
            ],
            ).unwrap();
    }

    pub fn get_sens(&self) -> u8 {
        self.sens
    }

    pub fn set_sens(&mut self, sens: u8) {
        self.sens = sens;
        self.analog_write(0, sens as u16);
    }

    pub fn read(&mut self) -> [u16; NUM_SENSORS] {
        const START: u8 = (8 - NUM_SENSORS as u8) / 2 + 1;
        const END: u8 = START + NUM_SENSORS as u8;
        debug_assert_eq!(END - START, NUM_SENSORS as u8);
        let vals: [u16; NUM_SENSORS] = unsafe { from_iter((START..END)
                                                          .into_iter()
                                                          .zip(self.correction)
                                                          .map(|(p, c)| ((self.analog_read(p) as i32) + c as i32) as u16))
                                                    .unwrap_unchecked() };
        vals
    }

    pub fn calibrate(&mut self, delay: &mut impl DelayMs<u16>) {
        const NUM_SAMPLES: usize = 20;

        let vals = self.read();
        rprintln!("Before calibration: corr={:?} vals={:?}", self.correction, vals);

        let mut samples = [[0_u16; NUM_SAMPLES]; NUM_SENSORS];
        for i in 0..NUM_SAMPLES {
            let vals = self.read();
            vals
                .iter()
                .zip(samples
                     .iter_mut())
                .for_each(|(&val, s)| s[i] = val);
            delay.delay_ms(25);
        }
        let vals_averaged = samples
            .map(|vals| vals
                         .into_iter()
                         .map_into::<u32>()
                         .sum::<u32>() / vals.len() as u32);
        let mean = vals_averaged
                    .into_iter()
                    .sum::<u32>() / vals_averaged.len() as u32;
        self.correction = vals_averaged.map(|x| mean as i32 - x as i32);

        let vals = self.read();
        rprintln!("After calibration: corr={:?} vals{:?} mean={}", self.correction, vals, mean);
    }
}

