"""
Turns an XML with intrinsics into a nice list of "globs" like this:

# AMX
_tile_{{dpfp16ps,dpbsud,release,zero,cmmimfp16ps,loadd,dpbuud,storeconfig,dpbssd,dpbusd,cmmrlfp16ps,dpbf16ps,loadconfig,stored},stream_loadd}
__tile_{{loadd,dpbuud,dpbf16ps,zero,dpbusd,dpbssd,dpfp16ps,stored,cmmimfp16ps,cmmrlfp16ps,dpbsud},stream_loadd}
# Other
...

This is a low effort script to balance file size and diff size
"""

import sys
import xml.etree.ElementTree as ET
from collections import defaultdict

from ._glob import globify

if len(sys.argv) != 2:
    sys.stderr.write("intbuild.py: expected exactly 1 CLI argument, the data.xml path\n")
    sys.exit(1)

tree = ET.parse(sys.argv[1])
root = tree.getroot()
assert root.tag == "intrinsics_list", "The XML is in an unexpected format"

techs: dict[str, set[str]] = defaultdict(set)

for child in root.iter("intrinsic"):
    name = child.get("name")
    tech = child.get("tech")
    assert name
    assert tech, f"tech not found in intrinsic {name!r}"

    assert not (set(name) & set("{},")), f"invalid name {name!r}"

    if (name, tech) != ("_get_ssp", "Other"):
        # not sure what's up with this, but it's been superseded by `_rdsspd_i32` and `_rdsspd_i64`,
        # and it's unrelated to SIM, so we just ignore it
        assert name not in techs[tech], f"duplicate intrinsic {name!r} in {tech!r}"

    techs[tech].add(name)

for tech, intrinsics in techs.items():
    print(f"# {tech}")
    for i in globify(intrinsics):
        print(i)
