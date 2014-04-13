﻿using Pozyx.CAE.Lib.CellSpaces;
using System.Runtime.InteropServices;

namespace Pozyx.CAE.Lib.Runners
{
    public class ThreadPerCellStepCpuSyncedGpuRunner : StepCpuSyncedRunner<IntArrayCellSpace, int>
    {
        [DllImport("Pozyx.CAE.Lib.AMP.dll", CallingConvention = CallingConvention.StdCall)]
        extern unsafe private static void ApplyRuleOneStepGpu(
            int* inputCellSpaceBytes, int inputCellSpaceLength,
            int* outputCellSpaceBytes, int outputCellSpaceLength,
            int offsetDifference, byte rule);

        unsafe protected override void RunStep(IntArrayCellSpace inputCellSpace, IntArrayCellSpace outputCellSpace, bool[] rule)
        {
            var ruleByte = RuleTools.ConvertBitsToByte(rule);

            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            fixed (int* inputCellSpaceBytes = &inputCellSpace.Cells[0],
                        outputCellSpaceBytes = &outputCellSpace.Cells[0])
            {
                ApplyRuleOneStepGpu(
                    inputCellSpaceBytes, inputCellSpace.Length,
                    outputCellSpaceBytes, outputCellSpace.Length,
                    offsetDifference, ruleByte);
            }
        }
    }
}
