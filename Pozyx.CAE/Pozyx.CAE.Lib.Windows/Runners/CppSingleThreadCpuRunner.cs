using Pozyx.CAE.Lib.CellSpaces;
using System.Runtime.InteropServices;

namespace Pozyx.CAE.Lib.Runners
{
    public class CppSingleThreadCpuRunner : StepCpuSyncedRunner<BoolArrayCellSpace>
    {
        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe private static void ApplyRuleOneStepSingleThreadCpu(
            bool* inputCellSpace, int inputCellSpaceLength,
            bool* outputCellSpace, int outputCellSpaceLength,
            int offsetDifference, byte rule);

        unsafe protected override void RunStep(BoolArrayCellSpace inputCellSpace, BoolArrayCellSpace outputCellSpace, bool[] rule)
        {
            var ruleByte = RuleTools.ConvertBitsToByte(rule);

            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            fixed (bool* inputCellSpaceBools = &inputCellSpace.Cells[0],
                         outputCellSpaceBools = &outputCellSpace.Cells[0])
            {
                ApplyRuleOneStepSingleThreadCpu(
                    inputCellSpaceBools, inputCellSpace.Length,
                    outputCellSpaceBools, outputCellSpace.Length,
                    offsetDifference, ruleByte);
            }
        }
    }
}
