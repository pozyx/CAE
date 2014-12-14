#include "amp.h"
#include "common.cpp"

using namespace std;
using namespace concurrency;

extern "C" __declspec (dllexport) int _stdcall ApplyRuleOneStepGpuWithLimitedConcurrency(
	int* inputCellSpace, int inputCellSpaceLength,
	int* outputCellSpace, int outputCellSpaceLength,
	int offsetDifference, byte rule, int maxConcurrency)
{
	int iterationsPerThread = outputCellSpaceLength / maxConcurrency;

	vector<int> startOutIndexes;
	vector<int> endOutIndexes;	

	for (int i = 0; i < maxConcurrency; i++)
	{
		int startOutIndex = i * iterationsPerThread;

		int endOutIndex =
			i == maxConcurrency - 1 ?
			outputCellSpaceLength :
			startOutIndex + iterationsPerThread;

		if (endOutIndex - startOutIndex > 0)
		{
			startOutIndexes.push_back(startOutIndex);
			endOutIndexes.push_back(endOutIndex);
		}
	}

	array_view<const int, 1> startOutIndexesArray((int) startOutIndexes.size(), startOutIndexes);
	array_view<const int, 1> endOutIndexesArray((int) endOutIndexes.size(), endOutIndexes);

	array_view<const int, 1> inputCellSpaceArray(inputCellSpaceLength, inputCellSpace);

	array_view<int, 1> outputCellSpaceArray(outputCellSpaceLength, outputCellSpace);
	outputCellSpaceArray.discard_data();

	int intRule = (int)rule;

	parallel_for_each(startOutIndexesArray.extent, [=](index<1> idx) restrict(amp)
	{
		int indexToOutIndexArrays = idx[0];
		
		for (int outIndex = startOutIndexesArray(indexToOutIndexArrays); outIndex < endOutIndexesArray(indexToOutIndexArrays); outIndex++)
		{
			int inIndex = outIndex + offsetDifference;

			bool oldLeftValue = inIndex - 1 >= 0 && inIndex - 1 < inputCellSpaceLength && inputCellSpaceArray(inIndex - 1);
			bool oldValue = inIndex >= 0 && inIndex < inputCellSpaceLength && inputCellSpaceArray(inIndex);
			bool oldRightValue = inIndex + 1 >= 0 && inIndex + 1 < inputCellSpaceLength && inputCellSpaceArray(inIndex + 1);

			outputCellSpaceArray[outIndex] = APPLY_RULE(intRule, oldLeftValue, oldValue, oldRightValue);
		}
	});

	outputCellSpaceArray.synchronize();

	return 0;
}