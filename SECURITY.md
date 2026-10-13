# Security policy

## Reporting a vulnerability

Report a vulnerability in private, through **Report a vulnerability** on the
[Security tab](https://github.com/linouxis9/oxirush-aper/security) of the
repository. Where that tab does not offer it, the
[Security tab of oxirush-ngap](https://github.com/linouxis9/oxirush-ngap/security),
which is built on this crate, takes the report. Please do not open a public
issue for it.

Say what the input is and what happens: the octets, the type or the fields
that were read, and the version or the commit of the crate. Octets that make
a reader panic, loop or allocate without bound are a vulnerability, and so is
a value that is written as octets which the reader gives another value for.

## Supported versions

The latest version on crates.io receives the fixes.
