from dataclasses import dataclass
from math import atan, tan, cos, acos, radians

from solid import *
from solid.utils import * 

gears = import_scad("./third-party/gears/gears.scad")


@dataclass
class Gear:
    CLEARANCE = 0.05

    modul: float
    tooth_number: int
    width: float
    bore: float
    pressure_angle: float
    helix_angle: float
    optimized: bool

    def __call__(self) -> Any:
        return gears.spur_gear(self.modul,
                               self.tooth_number,
                               self.width,
                               self.bore,
                               self.pressure_angle,
                               self.helix_angle,
                               self.optimized)

    @property
    def pitch_diam(self):
        return self.modul * self.tooth_number

    @property
    def pitch_radius(self):
        return self.pitch_diam / 2

    @property
    def alpha_spur(self):
        # Helix Angle in Transverse Section
        return atan(tan(radians(self.pressure_angle)) / cos(radians(self.helix_angle)))

    @property
    def base_diam(self):
        return self.pitch_diam * cos(self.alpha_spur)

    @property
    def base_radius(self):
        return self.base_diam / 2

    @property
    def tip_diam(self):
        # according to DIN 58400 or DIN 867
        return self.pitch_diam + self.modul * 2.2 if (self.modul < 1) else self.pitch_diam + self.modul * 2

    @property
    def tip_radius(self):
        return self.tip_diam / 2

    @property
    def tip_clearance(self):
        return 0 if (self.tooth_number < 3) else self.modul / 6

    @property
    def root_diam(self):
        return self.pitch_diam - 2 * (self.modul + self.tip_clearance)

    @property
    def root_radius(self):
        return self.root_diam / 2

    @property
    def max_rolling_angle(self):
        # Involute begins on the Base Circle and ends at the Tip Circle
        return acos(self.base_radius / self.tip_radius)

    @property
    def pitch_rolling_angle(self):
        # Involute begins on the Base Circle and ends at the Tip Circle
        return acos(self.base_radius / self.pitch_radius)

    @property
    def torsion_angle(self):
        # for Extrusion
        return degrees(self.width / (self.pitch_radius * tan(degrees(90 - self.helix_angle))))

    @property
    def pitch_angle(self):
        return 360 / self.tooth_number

    @property
    def phi_r(self):
        # Angle to Point of Involute on Pitch Circle
        return degrees(tan(self.max_rolling_angle) - self.max_rolling_angle)

    @property
    def tooth_width(self):
        return (180 * (1 + self.CLEARANCE)) / self.tooth_number + 2 * self.phi_r;

    def mesh(self, b):
        return right(self.pitch_radius + b.pitch_radius) \
                     (rotate((0, 0, 180 / b.tooth_number * (1 if b.tooth_number % 2 == 0 else 2))) \
                             (b()
                             )
                     )


