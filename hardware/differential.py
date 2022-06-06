from dataclasses import dataclass
from math import asin, atan, tan, cos, acos, radians
import math

from solid import *
from solid.utils import * 
import numpy as np


gears = import_scad("./third-party/gears/gears.scad")

GEAR_MODULE = 2.5
PRESSURE_ANGLE = 25

OUT_GEARS_DISTANCE = 22
PLANET_GEARS_MESH_HEIGHT = 20


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
        return atan(tan(self.pressure_angle)/cos(self.helix_angle))

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
        return degrees(self.width / (self.pitch_radius * tan(90 - self.helix_angle)))

    @property
    def pitch_angle(self):
        return 360 / self.tooth_number

    @property
    def phi_r(self):
        # Angle to Point of Involute on Pitch Circle
        return degrees(tan(self.max_rolling_angle) - radians(self.max_rolling_angle))

    @property
    def tooth_width(self):
        return (180 * (1 + self.CLEARANCE)) / self.tooth_number + 2 * self.phi_r;

    def mesh(self, b):
        return right(self.pitch_radius + b.pitch_radius) \
                     (rotate((0, 0, 180 / b.tooth_number * (1 if b.tooth_number % 2 == 0 else 2))) \
                             (b()
                             )
                     )


def half(is_lower: bool):
    def out_gear(is_lower: bool):
        return Gear(modul=GEAR_MODULE, tooth_number=30,
                    width=30, bore=8,
                    pressure_angle=PRESSURE_ANGLE,
                    helix_angle=25 * (1 if is_lower else -1),
                    optimized=False)

    def planet_gear(is_lower: bool):
        return Gear(modul=GEAR_MODULE, tooth_number=10,
                    width=30 + PLANET_GEARS_MESH_HEIGHT, bore=8,
                    pressure_angle=PRESSURE_ANGLE,
                    helix_angle=25 * (-1 if is_lower else 1),
                    optimized=False)

    sun = out_gear(is_lower)

    planets = []
    for i in range(5):
        planets.append(rotate(a=360 / 5 * i)(sun.mesh(planet_gear(True))))


    return sun() + sum(planets), planet_gear(True).pitch_radius / (sun.pitch_radius + planet_gear(True).pitch_radius)


def main():
    lower, x = half(True)

    return lower \
            + rotate(a=degrees(asin(x)) * 2)(up(30 * 2 + OUT_GEARS_DISTANCE)(mirror(UP_VEC)(lower)))

scad_render_to_file(main(), out_dir="export", include_orig_code=False, file_header="$fn=100;")
