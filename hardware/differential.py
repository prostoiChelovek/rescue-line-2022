from math import asin
import functools
from copy import copy, deepcopy

from solid import *
from solid.utils import * 

from gear import Gear


GEAR_MODULE = 2.5
PRESSURE_ANGLE = 25

OUT_GEARS_DISTANCE = 22
PLANET_GEARS_MESH_HEIGHT = 20
NUM_PLANETS = 5

GEAR_POCKET_TOLERANCE = 0.4


def Slot(root: Optional[OpenSCADObject] = None) -> OpenSCADObject:
    def find_root_child(root):
        res = []
        def _inner(el):
            if len(el.children) > 0:
                res.append(len(el.children) - 1)
                _inner(el.children[-1])
        _inner(root)
        return res
    def get_root_child(el, path):
        if len(path) > 0:
            return get_root_child(el.children[path[0]], path[1:])
        else:
            return el
    root = root or union()
    orig_add = root.add.__func__
    root._child_path = find_root_child(root)

    @functools.wraps(root.add)
    def add(self, o: OpenSCADObjectPlus) -> OpenSCADObject:
        new = deepcopy(self)
        if isinstance(o, Sequence):
            for obj in o:
                new.add(obj)
        elif isinstance(o, OpenSCADObject):
            if len(new._child_path) > 0:
                get_root_child(new, new._child_path).add(o)
            else:
                orig_add(new, o)
        else:
            raise TypeError
        return new

    def stack(self, slot):
        self.add(slot)
        self._child_path += [len(get_root_child(self, self._child_path).children)-1]
        return self

    name = f"{type(root).__name__}_Slot"
    root.__class__ = type(name, (type(root),),
            {"__call__": add, "add": add, "__add__": add, "stack": stack})
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

        transform = Slot(rotate(360 / NUM_PLANETS * i)(
                        forward(sun.pitch_radius + p.pitch_radius)))

        planets_r += transform(rotate((0, 0, mesh_rotation if not is_lower else 0))( p()))

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
