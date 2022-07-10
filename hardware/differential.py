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


def out_gear():
    return Gear(modul=GEAR_MODULE, tooth_number=30,
                width=30, bore=8,
                pressure_angle=PRESSURE_ANGLE,
                helix_angle=25,
                optimized=False)

def planet_gear():
    return Gear(modul=GEAR_MODULE, tooth_number=10,
                width=30 + PLANET_GEARS_MESH_HEIGHT, bore=8,
                pressure_angle=PRESSURE_ANGLE,
                helix_angle=25,
                optimized=False)


def assembly():
    lower_sun = out_gear()
    upper_sun = out_gear()
    upper_sun.helix_angle *= -1

    upper_r = Slot(up(upper_sun.width + OUT_GEARS_DISTANCE))
    suns_mesh_r = Slot(rotate((0, 0, lower_sun.mesh_rotation)))

    planets = union()

    for i in range(NUM_PLANETS):
        lower_planet = planet_gear()
        lower_planet.helix_angle *= -1
        upper_planet = planet_gear()

        planets_r = Slot(forward(lower_sun.pitch_radius + lower_planet.pitch_radius))
        planets_distance = upper_sun.pitch_radius + lower_planet.pitch_radius 
        planets_rotation = asin(lower_planet.pitch_radius / planets_distance)
        upper_planet_r = Slot(
                              rotate((0, 0, degrees(planets_rotation * 2)))(
                                  upper_r(
                                      planets_r(
                                          down(PLANET_GEARS_MESH_HEIGHT)
                                      )
                                  )
                              )
                              )
        
        distance_from_bot = OUT_GEARS_DISTANCE - PLANET_GEARS_MESH_HEIGHT
        torsion_compensation = upper_planet.torsion_angle * (distance_from_bot / upper_planet.width)
        upper_mesh_rotation = upper_planet.mesh_rotation + torsion_compensation

        planets += rotate(360 / NUM_PLANETS * i)(
                          planets_r(lower_planet()),
                          upper_planet_r(
                                         rotate((0, 0, upper_mesh_rotation))(
                                                upper_planet()
                                               )
                                        )
                         )

    return union()(
                   suns_mesh_r(lower_sun()),
                   suns_mesh_r(upper_r(upper_sun())),
                   planets)


def main():
    return assembly()


if __name__ == "__main__":
    scad_render_to_file(main(), out_dir="export",
                        include_orig_code=False,
                        file_header="$fn=100;")
