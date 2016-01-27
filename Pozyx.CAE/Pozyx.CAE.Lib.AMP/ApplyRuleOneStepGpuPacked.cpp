#include "amp.h"
#include "common.cpp"

using namespace concurrency;

extern "C" __declspec (dllexport) int _stdcall ApplyRuleOneStepGpuPacked(
	int* inputCellSpace, int inputCellSpaceLength,
	int* outputCellSpace, int outputCellSpaceLength,
	int offsetDifference, byte rule)
{
	int inputCellSpaceArrayLength = (int) ceil((double)inputCellSpaceLength / BITS_IN_INT);
	int outputCellSpaceArrayLength = (int) ceil((double)outputCellSpaceLength / BITS_IN_INT);

	array_view<const int, 1> inputCellSpaceArray(inputCellSpaceArrayLength, inputCellSpace);

	array_view<int, 1> outputCellSpaceArray(outputCellSpaceArrayLength, outputCellSpace);
	outputCellSpaceArray.discard_data();

	int intRule = (int)rule;

	parallel_for_each(outputCellSpaceArray.extent, [=](index<1> idx) restrict(amp)
	{
		int outArrayIndex = idx[0];

		for (int outIntIndex = 0; outIntIndex < BITS_IN_INT; outIntIndex++)
		{
			int outIndex = (outArrayIndex * BITS_IN_INT) + outIntIndex;
			int inIndex = outIndex + offsetDifference;

			bool oldLeftValue = inIndex - 1 >= 0 && inIndex - 1 < inputCellSpaceLength && CHECK_BIT(inputCellSpaceArray(ARRAY_INDEX(inIndex - 1)), INT_INDEX(inIndex - 1));
			bool oldValue = inIndex >= 0 && inIndex < inputCellSpaceLength && CHECK_BIT(inputCellSpaceArray(ARRAY_INDEX(inIndex)), INT_INDEX(inIndex));
			bool oldRightValue = inIndex + 1 >= 0 && inIndex + 1 < inputCellSpaceLength && CHECK_BIT(inputCellSpaceArray(ARRAY_INDEX(inIndex + 1)), INT_INDEX(inIndex + 1));

			if (APPLY_RULE(intRule, oldLeftValue, oldValue, oldRightValue))
				outputCellSpaceArray[idx] |= (1 << INT_INDEX(outIntIndex));
			else
				outputCellSpaceArray[idx] &= ~(1 << INT_INDEX(outIntIndex));
		}
	});

	outputCellSpaceArray.synchronize();

	return 0;
}