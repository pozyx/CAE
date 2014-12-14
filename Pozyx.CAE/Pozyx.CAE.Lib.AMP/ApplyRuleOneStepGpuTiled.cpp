#include "amp.h"
#include "common.cpp"

using namespace concurrency;

extern "C" __declspec (dllexport) int _stdcall ApplyRuleOneStepGpuTiled(
	int* inputCellSpace, int inputCellSpaceLength,
	int* outputCellSpace, int outputCellSpaceLength,
	int offsetDifference, byte rule)
{
	// TODO: experiment with different tile sizes:
	// - up to 1024
	// - not smaller than warp size (32)
	// - multiple of warp size (32)
	// - occupancy (less can be more)

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

		tile_static int oldValues[TileSize];

		oldValues[tidx.local[0]] = inIndex >= 0 && inIndex < inputCellSpaceLength && inputCellSpaceArray(inIndex);

		tidx.barrier.wait();

		bool oldLeftValue;

		if (tidx.local[0]- 1 >= 0 && tidx.local[0] - 1 < TileSize)
		{
			oldLeftValue = oldValues[tidx.local[0] - 1];
		}
		else
		{
			oldLeftValue = inIndex - 1 >= 0 && inIndex - 1 < inputCellSpaceLength && inputCellSpaceArray(inIndex - 1);
		}

		bool oldValue = oldValues[tidx.local[0]];

		bool oldRightValue;

		if (tidx.local[0] + 1 >= 0 && tidx.local[0] + 1 < TileSize)
		{
			oldRightValue = oldValues[tidx.local[0] + 1];
		}
		else
		{
			oldRightValue = inIndex + 1 >= 0 && inIndex + 1 < inputCellSpaceLength && inputCellSpaceArray(inIndex + 1);
		}

		outputCellSpaceArray[tidx] = APPLY_RULE(intRule, oldLeftValue, oldValue, oldRightValue);
	});

	outputCellSpaceArray.synchronize();

	return 0;
}