#!/usr/bin/env python3
"""Write tests/readme.rs of a protocol crate from its README: the test that each Rust
block of the README prints what the README shows under it.

    python3 ../oxirush-aper/protocols/tests/readme.py           # write it
    python3 ../oxirush-aper/protocols/tests/readme.py --check   # compare it

Run it from the directory of the crate. The Rust blocks of the README are documentation
tests: they compile and run, and nothing looks at what they print. The file that this
writes has each of them as a function that prints into a string, and the output that the
README shows under it, which its test compares with what the function prints. After a
change of a Rust block of the README or of what it shows, write the file again and run
the test; CI runs this with `--check`, which fails while the file is not the one of the
README.

The test is compiled with the features that `required-features` of the `[[test]]` named
`readme` of Cargo.toml names, and always where none is named.

The output of a block is the `json` and `text` blocks that follow it with nothing but
blank lines between them. A block that shows no output prints none. A block that has a
`main` is run as the documentation test runs it: its function calls `main`, which returns
a `Result`, and an error fails the test. The blocks go by their rank, and not by their
line: a change of the prose of the README leaves the file as it is.
"""
import pathlib
import sys
import tomllib

ROOT = pathlib.Path.cwd()
TEST = ROOT / "tests/readme.rs"

HEAD = '''\
// The Rust blocks of README.md, each as a function, with what the README shows under
// each: `../oxirush-aper/protocols/tests/readme.py` writes this file from the README and
// says how. Do not edit it.
{gate}#![allow(unused)]

use std::cell::RefCell;

thread_local!(static PRINTED: RefCell<String> = const { RefCell::new(String::new()) });

// What a block prints is kept, to be compared with what the README shows. The line
// is written before it is kept, so that `?` in what is printed returns from the block.
macro_rules! print {
    ($($argument:tt)*) => {{
        let text = format!($($argument)*);
        PRINTED.with(|printed| printed.borrow_mut().push_str(&text))
    }};
}

macro_rules! println {
    () => {
        print!("\\n")
    };
    ($($argument:tt)*) => {{
        let line = format!($($argument)*);
        print!("{line}\\n")
    }};
}

/// What `block` prints.
fn printed(block: fn()) -> String {
    PRINTED.with(|printed| printed.borrow_mut().clear());
    block();
    PRINTED.with(|printed| printed.take())
}

#[test]
fn each_block_prints_what_the_readme_shows() {
    let mut differences = String::new();
    for (rank, (block, shown)) in (1..).zip(BLOCKS) {
        let printed = printed(*block);
        if printed != *shown {
            differences += &format!("the Rust block {rank} of README.md prints:\\n{printed}");
            differences += &format!("and the README shows:\\n{shown}\\n");
        }
    }
    assert!(differences.is_empty(), "{differences}");
}
'''


def gate(manifest):
    """The attribute that compiles the test with the features that `manifest`, the text of
    Cargo.toml, has the `[[test]]` named `readme` require, and none where it names none.

    >>> print(gate('[[test]]\\nname = "readme"\\nrequired-features = ["tree"]'), end="")
    #![cfg(feature = "tree")]
    >>> print(gate('[[test]]\\nname = "readme"\\nrequired-features = ["tree", "endpoint"]'), end="")
    #![cfg(all(feature = "tree", feature = "endpoint"))]
    >>> gate('[package]\\nname = "crate"'), gate('[[test]]\\nname = "other"\\nrequired-features = ["x"]')
    ('', '')
    """
    tests = tomllib.loads(manifest).get("test", [])
    readme = next((test for test in tests if test.get("name") == "readme"), {})
    features = readme.get("required-features", [])
    conditions = ", ".join(f'feature = "{feature}"' for feature in features)
    if len(features) > 1:
        conditions = f"all({conditions})"
    return f"#![cfg({conditions})]\n" if features else ""


def examples(readme):
    """The Rust blocks of the README, each as its code and the output shown under it."""
    found, open_, output = [], None, False
    # The lines as Rust reads them: after each line feed, without a carriage return.
    lines = readme.split("\n")
    if lines[-1] == "":
        lines.pop()
    for line in (line.removesuffix("\r") for line in lines):
        fence = line[3:] if line.startswith("```") else None
        if open_ is None and fence == "rust":
            open_, output = "rust", True
            found.append(["", ""])
        elif open_ is None and fence is not None:
            open_ = fence
            output = output and fence in ("json", "text")
        elif open_ is not None and fence == "":
            open_ = None
        elif open_ == "rust":
            found[-1][0] += line + "\n"
        elif open_ is not None:
            if output:
                found[-1][1] += line + "\n"
        else:
            # Text between two blocks ends the output of the first.
            output = output and not line.strip()
    return found


def literal(text):
    """`text` as a raw string literal of Rust."""
    hashes = 1
    while '"' + "#" * hashes in text:
        hashes += 1
    return f'r{"#" * hashes}"{text}"{"#" * hashes}'


def generated(found, manifest):
    """The text of tests/readme.rs for the Rust blocks of the README, and the crate whose
    Cargo.toml is `manifest`."""
    file = HEAD.replace("{gate}", gate(manifest))
    file += "\n/// Each Rust block of README.md, by its rank, and what the README shows under it.\n"
    file += "#[rustfmt::skip]\nconst BLOCKS: &[(fn(), &str)] = &[\n"
    for rank, (_, output) in enumerate(found, 1):
        file += f"    (block_{rank}, {literal(output)}),\n"
    file += "];\n"
    for rank, (code, _) in enumerate(found, 1):
        file += f"\n/// The Rust block {rank} of README.md.\n#[rustfmt::skip]\nfn block_{rank}() {{\n"
        for line in code.splitlines():
            file += f"    {line}\n" if line else "\n"
        # The `main` of a block is what the block does.
        if any(line.startswith("fn main(") for line in code.splitlines()):
            file += "\n    main().unwrap();\n"
        file += "}\n"
    return file


def main():
    if sys.argv[1:] not in ([], ["--check"]):
        raise SystemExit(__doc__)
    text = generated(examples((ROOT / "README.md").read_text()), (ROOT / "Cargo.toml").read_text())
    if sys.argv[1:] == ["--check"]:
        if not TEST.exists() or TEST.read_text() != text:
            raise SystemExit(
                "tests/readme.rs is not the Rust blocks of the README: write it with "
                "../oxirush-aper/protocols/tests/readme.py, then run its test"
            )
        return
    TEST.write_text(text)


if __name__ == "__main__":
    main()
