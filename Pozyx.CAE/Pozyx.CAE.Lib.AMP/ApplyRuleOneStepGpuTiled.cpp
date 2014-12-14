#include "amp.h"
#include "common.cpp"

using namespace concurrency;

extern "C" __declspec (dllexport) int _stdcall ApplyRuleOneStepGpuTiled(
	int* inputCellSpace, int inputCellSpaceLength,
	int* outputCellSpace, int outputCellSpaceLength,
	int offsetDifference, byte rule)
{
	static const int TileSize = 1024;

	// Cell space lengths must be multiple of tile size
	if ((inputCellSpaceLength % TileSize != 0) ||
		(outputCellSpaceLength % TileSize != 0))
	{
		return -1;
	}

	array_view<const int, 1> inputCellSpaceArray(inputCellSpaceLength, inputCellSpace);

	array_view<int, 1> outputCellSpaceArray(outputCellSpaceLength, outputCellSpace);
	outputCellSpaceArray.discard_data();

	int intRule = (int)rule;

	parallel_for_each(outputCellSpaceArray.extent.tile<TileSize>(), [=](tiled_index<TileSize> tidx) restrict(amp)
	{
		int outIndex = tidx.global[0];
		int inIndex = outIndex + offsetDifference;
		int localIndex = tidx.local[0];

		tile_static int oldValues[TileSize];

		bool oldValue = inIndex >= 0 && inIndex < inputCellSpaceLength && inputCellSpaceArray(inIndex);
		oldValues[localIndex] = oldValue;

		bool oldLeftValue;
		bool isOldLeftValueInTile = localIndex - 1 >= 0 && localIndex - 1 < TileSize;
		if (!isOldLeftValueInTile)
		{
			oldLeftValue = inIndex - 1 >= 0 && inIndex - 1 < inputCellSpaceLength && inputCellSpaceArray(inIndex - 1);
		}

		bool oldRightValue;
		bool isOldRightValueInTile = localIndex + 1 >= 0 && localIndex + 1 < TileSize;
		if (!isOldRightValueInTile)
		{
			oldRightValue = inIndex + 1 >= 0 && inIndex + 1 < inputCellSpaceLength && inputCellSpaceArray(inIndex + 1);
		}

		tidx.barrier.wait();

		if (isOldLeftValueInTile)
		{
			oldLeftValue = oldValues[localIndex - 1];
		}

		if (isOldRightValueInTile)
		{
			oldRightValue = oldValues[localIndex + 1];
		}

		outputCellSpaceArray[tidx] = APPLY_RULE(intRule, oldLeftValue, oldValue, oldRightValue);
	});

	outputCellSpaceArray.synchronize();

	return 0;
}