## general
 - serialise instructions
 - add spawn delay + remap support to asm
 - various other comp triggers 
 - replace NOP with WAIT \<x>
 - do not skip unindented strings that are not routine identifiers

## commands
### SPAWN command and derivatives

`SPAWN group1 + <float> @ <remaps>`
float: delay
remaps: remap dict. e.g. 192:168,0:11
`+` before `@` always

### `TSPAWN`
Args: `TSPAWN <timer>, <float>, <routine>`
Starts the timer specified, and when it counts up to the specified time (the second argument), the given routine is called.
internally uses timer trigger

###  `TSTART`
Args: `TSTART <timer>`
Starts this timer.
internally uses time ctrl trigger
### `TSTOP`
Args `TSTOP <timer>`
stops this timer.
internally uses time ctrl trigger

### `SRAND` / `FRAND`
Args: `SRAND <routine>, <float>`
Args: `FRAND <routine>, <routine>, <float>`
Spawns/Forks routines based on the chance
internally uses random trigger

### `RET`
Args: None
Returns from this routine.
internally uses stop trigger with ctrl id

all spawn triggers have a ctrl id that is the same as the group they're spawning
return: stop trigger that stops all objects with that control id (all spawn triggers that activate that group, and by proxy, the group itself)

### `WAIT`
Args: `WAIT <int>`
**Set to deprecate `NOP`.**
Waits for the given amount of ticks.

