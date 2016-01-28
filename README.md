CAE (Cellular Automata Engine)
------------------------------
Engine for Elementary Cellular Automata (https://en.wikipedia.org/wiki/Elementary_cellular_automaton)

Technologies: c#, Rx, C++ AMP

### How to Debug & Run

Uncomment any runner in `Pozyx.CAE.TestApp\Program.cs`

**non-AMP (c#):**
* startup project: `Pozyx.CAE.TestApp`
* Debug => Debug configuration, F5; Run => Release configuration, CTRL+F5

**AMP (c++, GPU):**
* startup project: `Pozyx.CAE.Lib.AMP` 
* Debugging Command setting in C++ proj.: `$(OutDir)\Pozyx.CAE.TestApp.exe`
* Debugger Type & Amp Default Accelerator settings in C++ proj.: `Auto+Warp` (C++ CPU); `GPU Only+Warp` (C++ GPU); `Mixed+Warp` (.NET CPU and C++ CPU)
* Debug => Debug configuration, F5; Run => Release configuration, CTRL+F5
* manually rebuild when .NET code changes!
