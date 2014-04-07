using Pozyx.CAE.Lib.CellSpaces;
using System;

namespace Pozyx.CAE.Lib.Runners
{
    public class ThreadPerCellStepCpuSyncedGpuRunner : StepCpuSyncedRunner
    {
        protected override void RunStep(BoolArrayCellSpace inputCellSpace, BoolArrayCellSpace outputCellSpace, bool[] rule)
        {
            // TODO:
            throw new NotImplementedException();
        }
    }
}
