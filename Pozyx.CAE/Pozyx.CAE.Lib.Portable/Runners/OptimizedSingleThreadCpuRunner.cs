using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Runners
{
    public class OptimizedSingleThreadCpuRunner : StepCpuSyncedRunner<BoolArrayCellSpace, bool>
    {
        protected override void RunStep(BoolArrayCellSpace inputCellSpace, BoolArrayCellSpace outputCellSpace, bool[] rule)
        {
            RuleTools.ApplyRule(inputCellSpace, outputCellSpace, rule, 0, outputCellSpace.Length);
        }
    }
}
