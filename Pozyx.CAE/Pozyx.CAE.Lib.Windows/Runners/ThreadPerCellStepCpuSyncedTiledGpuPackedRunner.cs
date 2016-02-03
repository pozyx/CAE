using System;
using System.Runtime.InteropServices;
using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Runners
{
    public class ThreadPerCellStepCpuSyncedTiledGpuPackedRunner : StepCpuSyncedRunner<PaddedPackedIntArrayCellSpace>
    { 
        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe private static int ApplyRuleOneStepGpuPackedTiled(
            int* inputCellSpace, int inputCellSpaceArrayLength,
            int* outputCellSpace, int outputCellSpaceArrayLength,
            int offsetDifference, byte rule);

        unsafe protected override void RunStep(PaddedPackedIntArrayCellSpace inputCellSpace, PaddedPackedIntArrayCellSpace outputCellSpace, bool[] rule)
        {
            var ruleByte = RuleTools.ConvertBitsToByte(rule);

            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            int errorCode;

            fixed (int* inputCellSpaceInts = &inputCellSpace.PackedCells[0],
                        outputCellSpaceInts = &outputCellSpace.PackedCells[0])
            {
                errorCode = ApplyRuleOneStepGpuPackedTiled(
                    inputCellSpaceInts, inputCellSpace.PackedCells.Length,
                    outputCellSpaceInts, outputCellSpace.PackedCells.Length,
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
