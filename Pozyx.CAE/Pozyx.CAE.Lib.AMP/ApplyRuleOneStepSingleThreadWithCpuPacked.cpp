#include "common.cpp"

extern "C" __declspec (dllexport) int _stdcall ApplyRuleOneStepSingleThreadWithCpuPacked(
		int* inputCellSpace, int inputCellSpaceLength,
		int* outputCellSpace, int outputCellSpaceLength,
		int offsetDifference, unsigned char rule)
{
	for (int index = 0, inputIndex = offsetDifference;
		index < outputCellSpaceLength;
		index++, inputIndex++)
	{
		bool oldLeftValue = inputIndex - 1 >= 0 && inputIndex - 1 < inputCellSpaceLength && CHECK_BIT(*(inputCellSpace + ARRAY_INDEX(inputIndex - 1)), INT_INDEX(inputIndex - 1));
		bool oldValue = inputIndex >= 0 && inputIndex < inputCellSpaceLength && CHECK_BIT(*(inputCellSpace + ARRAY_INDEX(inputIndex)), INT_INDEX(inputIndex));
		bool oldRightValue = inputIndex + 1 >= 0 && inputIndex + 1 < inputCellSpaceLength && CHECK_BIT(*(inputCellSpace + ARRAY_INDEX(inputIndex + 1)), INT_INDEX(inputIndex + 1));

		if (APPLY_RULE(rule, oldLeftValue, oldValue, oldRightValue))
			*(outputCellSpace + ARRAY_INDEX(index)) |= (1 << INT_INDEX(index));
		else
			*(outputCellSpace + ARRAY_INDEX(index)) &= ~(1 << INT_INDEX(index));
	}

	return 0;
}