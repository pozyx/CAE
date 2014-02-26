using System;
using System.Collections.Generic;

namespace Pozyx.CAE.Lib.Runners
{
    public class ThreadPoolWorkItemPerCoreStepCpuRunner : WorkItemPerCoreStepCpuRunner
    {
        protected override void StartWorkItemsAndWait(IList<Action> actions)
        {
            ThreadingTools.StartThreadPoolWorkItemsAndWait(actions);
        }
    }
}
