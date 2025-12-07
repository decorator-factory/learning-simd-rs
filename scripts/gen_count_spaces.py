import random
from pathlib import Path

DOTDOT = Path(__file__).parent.parent

numbers: list[str] = []
total = 0

while total < 69420:
    pow = random.randrange(0, 64)
    num = str(random.randrange(0, 2**pow))
    numbers.append(num)
    total += len(num)

path = DOTDOT / "test_data" / "count_spaces_long_input.txt"
with open(path, "w") as file:
    _ = file.write(" ".join(numbers))
    _ = file.write("\n" + "1 2 3 4 5 6 7 8" * 20)
print("wrote", path)
