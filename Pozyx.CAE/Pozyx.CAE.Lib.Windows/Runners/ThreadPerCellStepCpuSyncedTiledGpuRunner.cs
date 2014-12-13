using Pozyx.CAE.Lib.CellSpaces;
using System.Runtime.InteropServices;

namespace Pozyx.CAE.Lib.Runners
{
    public class ThreadPerCellStepCpuSyncedTiledGpuRunner : StepCpuSyncedRunner<IntArrayCellSpace, int>
    {
        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe private static void ApplyRuleOneStepGpuTiled(
            int* inputCellSpace, int inputCellSpaceLength,
            int* outputCellSpace, int outputCellSpaceLength,
            int offsetDifference, byte rule);

        unsafe protected override void RunStep(IntArrayCellSpace inputCellSpace, IntArrayCellSpace outputCellSpace, bool[] rule)
        {
            var ruleByte = RuleTools.ConvertBitsToByte(rule);

            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            fixed (int* inputCellSpaceInts = &inputCellSpace.Cells[0],
                        outputCellSpaceInts = &outputCellSpace.Cells[0])
            {
                ApplyRuleOneStepGpuTiled(
                    inputCellSpaceInts, inputCellSpace.Length,
                    outputCellSpaceInts, outputCellSpace.Length,
                    offsetDifference, ruleByte);
            }
        }
    }
}
