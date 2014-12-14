using System;
using Pozyx.CAE.Lib.CellSpaces;
using System.Runtime.InteropServices;

namespace Pozyx.CAE.Lib.Runners
{
    public class ThreadPerCellStepCpuSyncedGpuRunner : StepCpuSyncedRunner<IntArrayCellSpace>
    {
        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe private static int ApplyRuleOneStepGpu(
            int* inputCellSpace, int inputCellSpaceLength,
            int* outputCellSpace, int outputCellSpaceLength,
            int offsetDifference, byte rule);

        unsafe protected override void RunStep(IntArrayCellSpace inputCellSpace, IntArrayCellSpace outputCellSpace, bool[] rule)
        {
            var ruleByte = RuleTools.ConvertBitsToByte(rule);

            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            int errorCode;

            fixed (int* inputCellSpaceInts = &inputCellSpace.Cells[0],
                        outputCellSpaceInts = &outputCellSpace.Cells[0])
            {
                errorCode = 
                    ApplyRuleOneStepGpu(
                    inputCellSpaceInts, inputCellSpace.Length,
                    outputCellSpaceInts, outputCellSpace.Length,
                    offsetDifference, ruleByte);
            }

            if (errorCode != 0)
                throw new InvalidOperationException(string.Format("Error returned from native code. Code: {0}", errorCode));
        }
    }
}
