using System;
using System.Runtime.InteropServices;
using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Runners
{
    public class ThreadPerCellStepCpuSyncedTiledGpuPackedRunner : StepCpuSyncedRunner<PaddedPackedIntArrayCellSpace>
    {
        // TODO: change to ApplyRuleOneStepGpuPackedTiled
        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe private static int ApplyRuleOneStepGpuPacked(
            int* inputCellSpace, int inputCellSpaceLength,
            int* outputCellSpace, int outputCellSpaceLength,
            int offsetDifference, byte rule);

        unsafe protected override void RunStep(PaddedPackedIntArrayCellSpace inputCellSpace, PaddedPackedIntArrayCellSpace outputCellSpace, bool[] rule)
        {
            var ruleByte = RuleTools.ConvertBitsToByte(rule);

            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            int errorCode;

            fixed (int* inputCellSpaceInts = &inputCellSpace.PackedCells[0],
                        outputCellSpaceInts = &outputCellSpace.PackedCells[0])
            {
                errorCode = ApplyRuleOneStepGpuPacked(
                    inputCellSpaceInts, inputCellSpace.Length, // TODO: change inputCellSpace.PackedCells.Length,
                    outputCellSpaceInts, outputCellSpace.Length, // TODO: change outputCellSpace.PackedCells.Length,
                    offsetDifference, ruleByte);
            }

            if (errorCode != 0)
                throw new InvalidOperationException($"Error returned from native code. Code: {errorCode}");
        }
    }
}
