using System;
using System.Runtime.InteropServices;
using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Runners
{
    public class CppSingleThreadCpuPackedIntRunner : StepCpuSyncedRunner<PackedIntArrayCellSpace>
    {
        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe private static int ApplyRuleOneStepSingleThreadWithCpuPacked(
            int* inputCellSpace, int inputCellSpaceLength,
            int* outputCellSpace, int outputCellSpaceLength,
            int offsetDifference, byte rule);

        unsafe protected override void RunStep(PackedIntArrayCellSpace inputCellSpace, PackedIntArrayCellSpace outputCellSpace, bool[] rule)
        {
            var ruleByte = RuleTools.ConvertBitsToByte(rule);

            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            int errorCode;

            fixed (int* inputCellSpaceBools = &inputCellSpace.PackedCells[0],
                        outputCellSpaceBools = &outputCellSpace.PackedCells[0])
            {
                errorCode = ApplyRuleOneStepSingleThreadWithCpuPacked(
                    inputCellSpaceBools, inputCellSpace.Length,
                    outputCellSpaceBools, outputCellSpace.Length,
                    offsetDifference, ruleByte);
            }

            if (errorCode != 0)
                throw new InvalidOperationException($"Error returned from native code. Code: {errorCode}");
        }
    }
}
