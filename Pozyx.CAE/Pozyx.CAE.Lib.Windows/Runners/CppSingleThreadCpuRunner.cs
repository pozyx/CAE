using System;
using System.Runtime.InteropServices;
using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Runners
{
    public class CppSingleThreadCpuRunner : StepCpuSyncedRunner<BoolArrayCellSpace>
    {
        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe private static int ApplyRuleOneStepSingleThreadCpu(
            bool* inputCellSpace, int inputCellSpaceLength,
            bool* outputCellSpace, int outputCellSpaceLength,
            int offsetDifference, byte rule);

        unsafe protected override void RunStep(BoolArrayCellSpace inputCellSpace, BoolArrayCellSpace outputCellSpace, bool[] rule)
        {
            var ruleByte = RuleTools.ConvertBitsToByte(rule);

            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            int errorCode;

            fixed (bool* inputCellSpaceBools = &inputCellSpace.Cells[0],
                         outputCellSpaceBools = &outputCellSpace.Cells[0])
            {
                errorCode = ApplyRuleOneStepSingleThreadCpu(
                    inputCellSpaceBools, inputCellSpace.Length,
                    outputCellSpaceBools, outputCellSpace.Length,
                    offsetDifference, ruleByte);
            }

            if (errorCode != 0)
                throw new InvalidOperationException($"Error returned from native code. Code: {errorCode}");
        }
    }
}
