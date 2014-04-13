using Pozyx.CAE.Lib.CellSpaces;
using System.Runtime.InteropServices;

namespace Pozyx.CAE.Lib.Runners
{
    public class CppSingleThreadCpuRunner : StepCpuSyncedRunner<BoolArrayCellSpace, bool>
    {
        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe private static void ApplyRuleOneStepSingleThreadCpu(
            bool* inputCellSpaceBytes, int inputCellSpaceLength,
            bool* outputCellSpaceBytes, int outputCellSpaceLength,
            int offsetDifference, byte rule);

        unsafe protected override void RunStep(BoolArrayCellSpace inputCellSpace, BoolArrayCellSpace outputCellSpace, bool[] rule)
        {
            var ruleByte = RuleTools.ConvertBitsToByte(rule);

            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            fixed (bool* inputCellSpaceBytes = &inputCellSpace.Cells[0],
                         outputCellSpaceBytes = &outputCellSpace.Cells[0])
            {
                ApplyRuleOneStepSingleThreadCpu(
                    inputCellSpaceBytes, inputCellSpace.Length,
                    outputCellSpaceBytes, outputCellSpace.Length,
                    offsetDifference, ruleByte);
            }
        }
    }
}
