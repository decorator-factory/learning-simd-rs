import base64
from pathlib import Path
import random

DOT = Path(__file__).parent
DOTDOT = Path(__file__).parent.parent

in_path = DOT / "b64_source.txt"

text = (
    in_path.read_bytes()
    + bytes(range(256))
    + bytes([0] * 64)
    + b"\n" * 32
    + bytes([random.randint(0, 255) for _ in range(4096)])
)

out_path_input = DOTDOT / "test_data" / "b64_input.bin"
_ = out_path_input.write_bytes(text)
print("wrote", out_path_input)

out_path_output = DOTDOT / "test_data" / "b64_expected_output.txt"
_ = out_path_output.write_bytes(base64.b64encode(text))
print("wrote", out_path_output)
