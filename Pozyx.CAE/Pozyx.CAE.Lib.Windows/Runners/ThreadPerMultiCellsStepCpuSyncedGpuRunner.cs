using System;
using System.Runtime.InteropServices;
using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Runners
{
    public class ThreadPerMultiCellsStepCpuSyncedGpuRunner : StepCpuSyncedRunner<IntArrayCellSpace>
    {
        private const int MaxConcurrency = 384;

        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe private static int ApplyRuleOneStepGpuWithLimitedConcurrency(
            int* inputCellSpace, int inputCellSpaceLength,
            int* outputCellSpace, int outputCellSpaceLength,
            int offsetDifference, byte rule, int maxConcurrency);

        unsafe protected override void RunStep(IntArrayCellSpace inputCellSpace, IntArrayCellSpace outputCellSpace, bool[] rule)
        {
            var ruleByte = RuleTools.ConvertBitsToByte(rule);

            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            int errorCode;

            fixed (int* inputCellSpaceInts = &inputCellSpace.Cells[0],
                        outputCellSpaceInts = &outputCellSpace.Cells[0])
            {
                errorCode = ApplyRuleOneStepGpuWithLimitedConcurrency(
                    inputCellSpaceInts, inputCellSpace.Length,
                    outputCellSpaceInts, outputCellSpace.Length,
                    offsetDifference, ruleByte, MaxConcurrency);
            }

            if (errorCode != 0)
                throw new InvalidOperationException($"Error returned from native code. Code: {errorCode}");
        }
    }
}
