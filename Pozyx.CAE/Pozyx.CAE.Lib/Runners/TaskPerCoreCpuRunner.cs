﻿using Pozyx.CAE.Lib.CellSpaces;
using System;
using System.Collections;
using System.Collections.Generic;
using System.Reactive.Disposables;
using System.Reactive.Linq;
using System.Reactive.Subjects;
using System.Threading;
using System.Threading.Tasks;

namespace Pozyx.CAE.Lib.Runners
{
    public class TaskPerCoreCpuRunner : IRunner<BoolArrayCellSpace>
    {
        public IConnectableObservable<BoolArrayCellSpace> Create(int ruleNumber, CancellationToken ct)
        {
            var rule = RuleTools.GetBoolArrayForRule(ruleNumber);

            return Observable.Create<BoolArrayCellSpace>(observer =>
            {
                Task.Run(() => Run(observer, rule, ct), ct)
                .ContinueWith(t =>
                {
                    if (t.IsCanceled)
                        observer.OnCompleted();
                    else
                        observer.OnError(t.Exception);
                },
                TaskContinuationOptions.NotOnRanToCompletion);

                return Disposable.Empty;
            })
            .Publish();
        }

        private static void Run(IObserver<BoolArrayCellSpace> observer, bool[] rule, CancellationToken ct)
        {
            var prevStep = new BoolArrayCellSpace();
            prevStep.Initialize(new BitArray(1, true), 0);
            observer.OnNext(prevStep);
            BoolArrayCellSpace nextStep;

            int? leftMostChangedIndex = 0;
            int? rightMostChangedIndex = 0;

            while (true)
            {
                ct.ThrowIfCancellationRequested();

                if (!leftMostChangedIndex.HasValue)
                {
                    observer.OnCompleted();
                    break;
                }

                var nextStepLength = rightMostChangedIndex.Value - leftMostChangedIndex.Value + 3;
                var nextStepOffset = leftMostChangedIndex.Value - 1;

                nextStep = new BoolArrayCellSpace();
                nextStep.Initialize(nextStepLength, nextStepOffset);

                var cellTasksForStep = new List<Task>(Environment.ProcessorCount);               

                var iterationsPerCore = nextStepLength / Environment.ProcessorCount;

                for (var i = 0; i < Environment.ProcessorCount; i++)
                {
                    var startIndex = nextStepOffset + (i * iterationsPerCore);

                    var endIndex = 
                        i == Environment.ProcessorCount - 1 ? 
                        nextStepOffset + nextStepLength :  
                        startIndex + iterationsPerCore;

                    if (endIndex - startIndex == 0)
                        continue;

                    cellTasksForStep.Add(
                         Task.Factory.StartNew(() =>
                         {
                             for (var index = startIndex; index < endIndex; index++)
                                 RuleTools.ApplyRule(prevStep, nextStep, index, rule);
                         },
                         TaskCreationOptions.AttachedToParent));
                }

                Task.WaitAll(cellTasksForStep.ToArray());

                observer.OnNext(nextStep);

                CellSpaceTools.GetChangeBounds(prevStep, nextStep, out leftMostChangedIndex, out rightMostChangedIndex);

                prevStep = nextStep;
            }
        }
    }
}

