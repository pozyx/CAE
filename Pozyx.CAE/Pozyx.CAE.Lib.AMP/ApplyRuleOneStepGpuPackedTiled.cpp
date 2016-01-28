#include "amp.h"
#include "common.cpp"

using namespace concurrency;

static const int TileSize = 1024;

int GetArrayValue(
	array_view<const int, 1> inputCellSpaceArray,
	int* inArrayValues,
	int inArrayIndex,
	int outArrayIndex,
	int mainInArrayValue,
	int localArrayIndex)
	restrict(amp)
{	
	int offset = inArrayIndex - outArrayIndex;
	
	int arrayValue;

	if (offset == 0)
		arrayValue = mainInArrayValue;
	else
	{
		bool isOldArrayValueInTile = localArrayIndex + offset >= 0 && localArrayIndex + offset < TileSize;

		if (isOldArrayValueInTile)
			arrayValue = inArrayValues[localArrayIndex + offset];			
		else
			arrayValue = inputCellSpaceArray(inArrayIndex);
	}

	return arrayValue;
}

extern "C" __declspec (dllexport) int _stdcall ApplyRuleOneStepGpuPackedTiled(
	int* inputCellSpace, int inputCellSpaceArrayLength,
	int* outputCellSpace, int outputCellSpaceArrayLength,
	int offsetDifference, byte rule)
{
	// Cell space lengths must be multiple of tile size
	if ((inputCellSpaceArrayLength % TileSize != 0) ||
		(outputCellSpaceArrayLength % TileSize != 0))
	{
		return -1;
	}

	array_view<const int, 1> inputCellSpaceArray(inputCellSpaceArrayLength, inputCellSpace);

	array_view<int, 1> outputCellSpaceArray(outputCellSpaceArrayLength, outputCellSpace);
	outputCellSpaceArray.discard_data();

	int intRule = (int)rule;

	int arrayOffsetDifference = offsetDifference / BITS_IN_INT;
	int inputCellSpaceLength = inputCellSpaceArrayLength * BITS_IN_INT;

	parallel_for_each(outputCellSpaceArray.extent.tile<TileSize>(), [=](tiled_index<TileSize> tidx) restrict(amp)
	{
		int outArrayIndex = tidx.global[0];
		int mainInArrayIndex = outArrayIndex + arrayOffsetDifference;
		int localArrayIndex = tidx.local[0];

		tile_static int inArrayValues[TileSize];

		int mainInArrayValue = inputCellSpaceArray(mainInArrayIndex);
		inArrayValues[localArrayIndex] = mainInArrayValue;

		tidx.barrier.wait();

		for (int outIntIndex = 0; outIntIndex < BITS_IN_INT; outIntIndex++)
		{
			int outIndex = (outArrayIndex * BITS_IN_INT) + outIntIndex;
			int inIndex = outIndex + offsetDifference;			
			
			int inLeftArrayIndex = ARRAY_INDEX(inIndex - 1);
			int inArrayIndex = ARRAY_INDEX(inIndex);
			int inRightArrayIndex = ARRAY_INDEX(inIndex + 1);

			int oldArrayValue = GetArrayValue(
				inputCellSpaceArray,
				inArrayValues,
				inArrayIndex,
				outArrayIndex,
				mainInArrayValue,
				localArrayIndex);
			
			int oldLeftArrayValue = GetArrayValue(
				inputCellSpaceArray,
				inArrayValues,
				inLeftArrayIndex,
				outArrayIndex,
				mainInArrayValue,
				localArrayIndex);

			int oldRightArrayValue = GetArrayValue(
				inputCellSpaceArray,
				inArrayValues,
				inRightArrayIndex,
				outArrayIndex,
				mainInArrayValue,
				localArrayIndex);

			bool oldLeftValue = inIndex - 1 >= 0 && inIndex - 1 < inputCellSpaceLength && CHECK_BIT(oldLeftArrayValue, INT_INDEX(inIndex - 1));
			bool oldValue = inIndex >= 0 && inIndex < inputCellSpaceLength && CHECK_BIT(oldArrayValue, INT_INDEX(inIndex));
			bool oldRightValue = inIndex + 1 >= 0 && inIndex + 1 < inputCellSpaceLength && CHECK_BIT(oldRightArrayValue, INT_INDEX(inIndex + 1));

			if (APPLY_RULE(intRule, oldLeftValue, oldValue, oldRightValue))
				outputCellSpaceArray[tidx] |= (1 << INT_INDEX(outIntIndex));
			else
				outputCellSpaceArray[tidx] &= ~(1 << INT_INDEX(outIntIndex));
		}
	});

	outputCellSpaceArray.synchronize();

	return 0;
}