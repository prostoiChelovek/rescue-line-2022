from math import asin
import functools

from solid import *
from solid.utils import * 

from gear import Gear


GEAR_MODULE = 2.5
PRESSURE_ANGLE = 25

OUT_GEARS_DISTANCE = 22
PLANET_GEARS_MESH_HEIGHT = 20
NUM_PLANETS = 5


def Slot(root: Optional[OpenSCADObject] = None) -> OpenSCADObject:
    root = root or union()
    add_fn = root.children[-1].add if len(root.children) > 0 else root.add

    @functools.wraps(root.add)
    def add(self, o: OpenSCADObjectPlus) -> OpenSCADObject:
        if isinstance(o, Sequence):
            for obj in o:
                self.add(obj)
        elif isinstance(o, OpenSCADObject):
            add_fn(o)
        else:
            raise TypeError
        return self

    name = f"{type(root).__name__}_Slot"
    root.__class__ = type(name, (type(root),),
                          {"add": add, "__add__": add})
    return root


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
    planets = [planet_gear(is_lower)] * NUM_PLANETS

    if is_lower:
        root = Slot()
        planets_r = Slot()
    else:
        root = Slot(up(sun.width + OUT_GEARS_DISTANCE))

        planets_distance = sun.pitch_radius + planet_gear(True).pitch_radius 
        planets_rotation = asin(planet_gear(True).pitch_radius / planets_distance)
        planets_r = Slot(down(PLANET_GEARS_MESH_HEIGHT)(
            rotate((0, 0, degrees(planets_rotation * 2)))
            )
            )

    for i, p in enumerate(planets):
        distance_from_bot = OUT_GEARS_DISTANCE - PLANET_GEARS_MESH_HEIGHT
        torsion_compensation = p.torsion_angle * (distance_from_bot / p.width)
        mesh_rotation = p.mesh_rotation + torsion_compensation
        planets_r += rotate(360 / NUM_PLANETS * i)(
                forward(sun.pitch_radius + p.pitch_radius)(
                    rotate((0, 0, mesh_rotation if not is_lower else 0))(
                        p())
                    )
                )

    root += planets_r

    root += rotate((0, 0, sun.mesh_rotation))(
                sun()
                )

    return root


def main():
    lower = half(True)
    upper = half(False)

    return lower + upper


if __name__ == "__main__":
    scad_render_to_file(main(), out_dir="export",
                        include_orig_code=False,
                        file_header="$fn=100;")
