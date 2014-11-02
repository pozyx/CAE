using Pozyx.CAE.Lib.CellSpaces;
using System.Runtime.InteropServices;

namespace Pozyx.CAE.Lib.Runners
{
    public class ThreadPerMultiCellsStepCpuSyncedGpuRunner : StepCpuSyncedRunner<IntArrayCellSpace, int>
    {
        private const int MaxConcurrency = 384;

        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe private static void ApplyRuleOneStepGpuWithLimitedConcurrency(
            int* inputCellSpace, int inputCellSpaceLength,
            int* outputCellSpace, int outputCellSpaceLength,
            int offsetDifference, byte rule, int maxConcurrency);

        unsafe protected override void RunStep(IntArrayCellSpace inputCellSpace, IntArrayCellSpace outputCellSpace, bool[] rule)
        {
            var ruleByte = RuleTools.ConvertBitsToByte(rule);

            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            fixed (int* inputCellSpaceInts = &inputCellSpace.Cells[0],
                        outputCellSpaceInts = &outputCellSpace.Cells[0])
            {
                ApplyRuleOneStepGpuWithLimitedConcurrency(
                    inputCellSpaceInts, inputCellSpace.Length,
                    outputCellSpaceInts, outputCellSpace.Length,
                    offsetDifference, ruleByte, MaxConcurrency);
            }
        }
    }
}
