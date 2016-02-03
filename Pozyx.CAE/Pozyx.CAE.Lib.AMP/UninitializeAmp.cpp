#include "amp.h"

using namespace concurrency;

extern "C" __declspec (dllexport) void _stdcall UninitializeAmp()
{
	concurrency::amp_uninitialize();
}