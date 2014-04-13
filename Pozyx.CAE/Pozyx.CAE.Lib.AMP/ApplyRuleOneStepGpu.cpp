#include "amp.h"
#include "common.h"

using namespace concurrency;

extern "C" __declspec (dllexport) void _stdcall ApplyRuleOneStepGpu(
	int* inputCellSpaceBytes, int inputCellSpaceLength,
	int* outputCellSpaceBytes, int outputCellSpaceLength,
	int offsetDifference, byte rule)
{
	array_view<const int, 1> inputCellSpaceArray(inputCellSpaceLength, inputCellSpaceBytes);

	array_view<int, 1> outputCellSpaceArray(outputCellSpaceLength, outputCellSpaceBytes);
	outputCellSpaceArray.discard_data();

	int intRule = (int)rule;

	parallel_for_each(outputCellSpaceArray.extent, [=](index<1> idx) restrict(amp)
	{
		int outIndex = idx[0];
		int inIndex = outIndex + offsetDifference;

		bool oldLeftValue = inIndex - 1 >= 0 && inIndex - 1 < inputCellSpaceLength && inputCellSpaceArray(inIndex - 1);
		bool oldValue = inIndex >= 0 && inIndex < inputCellSpaceLength && inputCellSpaceArray(inIndex);
		bool oldRightValue = inIndex + 1 >= 0 && inIndex + 1 < inputCellSpaceLength && inputCellSpaceArray(inIndex + 1);

		outputCellSpaceArray[idx] = APPLY_RULE(intRule, oldLeftValue, oldValue, oldRightValue);
	});

	outputCellSpaceArray.synchronize();
}