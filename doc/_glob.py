from collections import defaultdict
from collections.abc import Iterable, Iterator
import itertools


def unglob(s: str) -> list[str]:
    s = s.replace("[", "{").replace("]", ",}")  # HACK: `foo[optional]` -> `foo{optional,}`

    stack: list[list[list[str]]] = [[[""]]]

    for ch in s:
        if ch == "{":
            stack.append([[""]])
        elif ch == "}":
            postfix_alternates = stack.pop()
            prefix_alternates = stack[-1].pop()
            postfix_combos = list(itertools.product(postfix_alternates))
            rv: list[str] = []
            for b in prefix_alternates:
                for r in postfix_combos:
                    for u in r:
                        for h in u:
                            rv.append(b + h)
            stack[-1].append(rv)
        elif ch == ",":
            stack[-1].append([""])
        else:
            strings = stack[-1][-1]
            for i in range(len(strings)):
                strings[i] += ch
    return [x for xs in stack.pop() for x in xs]


def globify(names: Iterable[str]) -> Iterator[str]:
    trie = _Trie()
    for name in names:
        trie.push(name.split("_"))
    trie.sort()

    for p, t in _prefix_tries(trie):
        # this is one huge pile of spaghetti, i don't care
        yield "_".join([*p, t.to_glob()]).rstrip("_").replace("_,", ",")


def _prefix_tries(trie: "_Trie") -> Iterator[tuple[list[str], "_Trie"]]:
    parts: list[str] = []
    while len(trie._branches) == 1:
        [(k, v)] = trie._branches.items()
        parts.append(k)
        trie = v
    for k1, v in trie._branches.items():
        yield [*parts, k1], v


type _InfiniteDict = dict[str, _InfiniteDict]

class _Trie:
    def __init__(self) -> None:
        self._branches: dict[str, _Trie] = defaultdict(_Trie)

    def unwrap(self) -> _InfiniteDict:
        return {k: v.unwrap() for k, v in self._branches.items()}

    def sort(self) -> None:
        self._branches = {k: self._branches[k] for k in sorted(self._branches)}
        for v in self._branches.values():
            v.sort()

    def push(self, value: list[str]) -> None:
        if value:
            self._branches[value[0]].push(value[1:])

    def to_glob(self) -> str:
        if len(self._branches) == 0:
            return ""
        elif len(self._branches) == 1:
            [(k, subtrie)] = self._branches.items()
            if len(subtrie._branches) == 0:
                return k
            else:
                return k + "_" + subtrie.to_glob()
        else:
            # contract things like (apple_(foo|bar)|banana_(foo|bar)) to (apple_)
            glob_to_keys: dict[str, list[str]] = defaultdict(list)
            for key, sub in self._branches.items():
                glob = sub.to_glob()
                glob_to_keys[glob].append(key)

            chunks: list[str] = []
            for glob, keys in glob_to_keys.items():
                if len(keys) > 1:
                    prefix = "{" + ",".join(keys) + "}"
                else:
                    prefix = "".join(keys)

                if glob != "":
                    prefix += "_"
                chunks.append(prefix + glob)

            if len(chunks) > 1:
                return "{" +  ",".join(chunks) + "}"
            else:
                return "".join(chunks)
