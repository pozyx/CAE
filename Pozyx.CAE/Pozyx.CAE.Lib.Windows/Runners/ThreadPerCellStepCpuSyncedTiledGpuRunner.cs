using System;
using System.Runtime.InteropServices;
using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Runners
{
    public class ThreadPerCellStepCpuSyncedTiledGpuRunner : StepCpuSyncedRunner<PaddedIntArrayCellSpace>
    {
        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe private static int ApplyRuleOneStepGpuTiled(
            int* inputCellSpace, int inputCellSpaceLength,
            int* outputCellSpace, int outputCellSpaceLength,
            int offsetDifference, byte rule);

        unsafe protected override void RunStep(PaddedIntArrayCellSpace inputCellSpace, PaddedIntArrayCellSpace outputCellSpace, bool[] rule)
        {
            var ruleByte = RuleTools.ConvertBitsToByte(rule);

            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            int errorCode;

            fixed (int* inputCellSpaceInts = &inputCellSpace.Cells[0],
                        outputCellSpaceInts = &outputCellSpace.Cells[0])
            {
                errorCode = ApplyRuleOneStepGpuTiled(
                    inputCellSpaceInts, inputCellSpace.Cells.Length,
                    outputCellSpaceInts, outputCellSpace.Cells.Length,
                    offsetDifference, ruleByte);
            }

            if (errorCode != 0)
                throw new InvalidOperationException($"Error returned from native code. Code: {errorCode}");
        }

        public override void Dispose()
        {
            AmpUninitializer.UninitializeAmp();
            base.Dispose();
        }
    }
}
