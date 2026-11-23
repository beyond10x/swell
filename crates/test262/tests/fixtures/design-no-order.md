# Fixture design without an order of delivery

`design.md` without its line `order delivery ...`: the runner must refuse it.

```swell-slices
dir test/harness/ 1
dir test/language/ 2
dir test/built-ins/ 3
dir test/built-ins/Number/ 4
dir test/built-ins/Atomics/ 14
dir test/intl402/ 21
feature generators 7
feature Atomics 14
feature intl-normative-optional 21
flag async 10
flag CanBlockIsTrue 14
exclude decorators stage3
exclude-slice 21 intl402
exclude-include agentHelper.js agents
```
