using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace Pozyx.CAE.Lib.Runners
{
    public class TaskPerCoreStepCpuRunner : WorkItemPerCoreStepCpuRunner
    {
        protected override void StartWorkItemsAndWait(IList<Action> actions)
        {
            Task.WaitAll(
                actions
                    .Select(a => Task.Factory.StartNew(a, TaskCreationOptions.AttachedToParent))
                    .ToArray());
        }
    }
}
