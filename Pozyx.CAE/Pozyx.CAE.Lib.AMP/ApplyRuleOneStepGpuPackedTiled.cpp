#include "amp.h"
#include "common.cpp"

using namespace concurrency;

// TODO: does not work
// - try after sizeof(int) fix
// - try comment out (simplified without tiling)
// - then uncomment

extern "C" __declspec (dllexport) int _stdcall ApplyRuleOneStepGpuPackedTiled(
	int* inputCellSpace, int inputCellSpaceArrayLength,
	int* outputCellSpace, int outputCellSpaceArrayLength,
	int offsetDifference, byte rule)
{
	static const int TileSize = 1024;

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
	int outputCellSpaceLength = outputCellSpaceArrayLength * BITS_IN_INT;

	parallel_for_each(outputCellSpaceArray.extent.tile<TileSize>(), [=](tiled_index<TileSize> tidx) restrict(amp)
	{
		int outArrayIndex = tidx.global[0];
		int inArrayIndex = outArrayIndex + arrayOffsetDifference;
		int localArrayIndex = tidx.local[0];

		tile_static int oldArrayValues[TileSize];

		int oldArrayValue = inArrayIndex >= 0 && inArrayIndex < inputCellSpaceArrayLength && inputCellSpaceArray(inArrayIndex);
		oldArrayValues[localArrayIndex] = oldArrayValue;

		tidx.barrier.wait();

		for (int outIntIndex = 0; outIntIndex < BITS_IN_INT; outIntIndex++)
		{
			int outIndex = (outArrayIndex * BITS_IN_INT) + outIntIndex;
			int inIndex = outIndex + offsetDifference;			

			int oldLeftArrayIndex = ARRAY_INDEX(inIndex - 1);
			int oldArrayIndex = ARRAY_INDEX(inIndex);
			int oldRightArrayIndex = ARRAY_INDEX(inIndex + 1);

			// should equal: oldArrayValue = inputCellSpaceArray(oldArrayIndex);
			
			int oldLeftArrayValue;	
			int oldLeftArrayIndexOffset = oldArrayIndex - oldLeftArrayIndex;
	
			if (oldLeftArrayIndexOffset == 0)
			{				
				oldLeftArrayValue = oldArrayValue;
			}
			else // if (oldLeftArrayIndexOffset == 1)
			{				
				bool isOldLeftArrayValueInTile = localArrayIndex - 1 >= 0 && localArrayIndex - 1 < TileSize;
				
				if (!isOldLeftArrayValueInTile)
					oldLeftArrayValue = inputCellSpaceArray(oldLeftArrayIndex);
				else
					oldLeftArrayValue = oldArrayValues[localArrayIndex - 1];
			}			

			int oldRightArrayValue;
			int oldRightArrayIndexOffset = oldRightArrayIndex - oldArrayIndex;

			if (oldRightArrayIndexOffset == 0)
			{
				oldRightArrayValue = oldArrayValue;
			}
			else // if (oldRightArrayIndexOffset == 1)
			{
				bool isOldRightArrayValueInTile = localArrayIndex + 1 >= 0 && localArrayIndex + 1 < TileSize;

				if (!isOldRightArrayValueInTile)
					oldRightArrayValue = inputCellSpaceArray(oldRightArrayIndex);
				else
					oldRightArrayValue = oldArrayValues[localArrayIndex + 1];
			}
						
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