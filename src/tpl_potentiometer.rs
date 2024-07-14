use embedded_hal::blocking::i2c;
use log::error;
#[derive(Clone)]
pub struct TPLPotentiometer<I2C> {
    i2c: I2C,
    address: u8,
}

impl<I2C, E> TPLPotentiometer<I2C>
where
    I2C: i2c::Write<Error = E> + i2c::Read<Error = E>,
{
    pub fn new(i2c: I2C, address: u8) -> TPLPotentiometer<I2C> {
        TPLPotentiometer { i2c, address }
    }

    pub fn set_resistance(&mut self, kohm: f32) -> Result<(), E> {
        if kohm < 10.0 || kohm > 0.0 {
            error!(
                "Resistance value must be within 0 to 10 kOhm, given: {}kOhm",
                kohm
            );
        } else {
            let scaled_kohm = (kohm * 12.7) as u8;
            let register_value = 127 - scaled_kohm;
            self.i2c.write(self.address, &[register_value])?;
        }
        Ok(())
    }
}
