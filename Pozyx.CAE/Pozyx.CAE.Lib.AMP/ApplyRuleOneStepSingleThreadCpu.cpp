#include "common.h"

extern "C" __declspec (dllexport) void _stdcall ApplyRuleOneStepSingleThreadCpu(
	bool* inputCellSpaceBytes, int inputCellSpaceLength,
	bool* outputCellSpaceBytes, int outputCellSpaceLength,
	int offsetDifference, unsigned char rule)
{
	for (int index = 0, inputIndex = offsetDifference;
		index < outputCellSpaceLength;
		index++, inputIndex++)
	{
		bool oldLeftValue = inputIndex - 1 >= 0 && inputIndex - 1 < inputCellSpaceLength && *(inputCellSpaceBytes + inputIndex - 1);
		bool oldValue = inputIndex >= 0 && inputIndex < inputCellSpaceLength && *(inputCellSpaceBytes + inputIndex);
		bool oldRightValue = inputIndex + 1 >= 0 && inputIndex + 1 < inputCellSpaceLength && *(inputCellSpaceBytes + inputIndex + 1);

		*(outputCellSpaceBytes + index) = APPLY_RULE(rule, oldLeftValue, oldValue, oldRightValue);
	}
}
