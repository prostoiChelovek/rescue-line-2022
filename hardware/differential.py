from math import asin

from solid import *
from solid.utils import * 

from gear import Gear


GEAR_MODULE = 2.5
PRESSURE_ANGLE = 25

OUT_GEARS_DISTANCE = 22
PLANET_GEARS_MESH_HEIGHT = 20


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
