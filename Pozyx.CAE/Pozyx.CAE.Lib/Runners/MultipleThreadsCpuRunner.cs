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
    // TODO: more efficient (no barriers, change signalling)
    // TODO: check correctness

    public class MultipleThreadsCpuRunner<TCellSpace> : IRunner<TCellSpace> where TCellSpace : ICellSpace, new()
    {                
        public IConnectableObservable<TCellSpace> Create(int ruleNumber, CancellationToken ct, Action threadInit = null)
        {            
            var rule = GetBitArrayForRule(ruleNumber);            

            return Observable.Create<TCellSpace>(observer =>
            {
                var innerCts = new CancellationTokenSource();                

                Task.Run(() =>
                {
                    TCellSpace prevStep = default(TCellSpace);
                    TCellSpace nextStep = default(TCellSpace);

                    var cellTasks = new Dictionary<int, Task>();

                    var beginBarrier = new Barrier(1);
                    var endBarrier = new Barrier(1);

                    var boundsSyncObj = new object();

                    prevStep = new TCellSpace();
                    prevStep.Initialize(new BitArray(1, true), 0);                    
                                                            
                    int? leftMostChangedIndex = 0;
                    int? rightMostChangedIndex = 0;                    

                    while (true)
                    {                        
                        ct.ThrowIfCancellationRequested();                        

                        if (!leftMostChangedIndex.HasValue)
                        {                            
                            observer.OnCompleted();
                            innerCts.Cancel();
                            break;
                        }                        

                        var nextStepLength = rightMostChangedIndex.Value - leftMostChangedIndex.Value + 3;
                        var nextStepOffset = leftMostChangedIndex.Value - 1;

                        nextStep = new TCellSpace();
                        nextStep.Initialize(new BitArray(nextStepLength), nextStepOffset);                        
                        
                        leftMostChangedIndex = null;
                        rightMostChangedIndex = null;                        

                        for (var index = nextStepOffset; index < nextStepLength + nextStepOffset; index++)
                        {                            
                            if (cellTasks.ContainsKey(index)) continue;                                                        

                            beginBarrier.AddParticipant();
                            endBarrier.AddParticipant();

                            var index1 = index;

                            cellTasks.Add(index, Task.Factory.StartNew(() =>
                            {
                                var index2 = index1;

                                if (threadInit != null)
                                    threadInit();                                                                

                                try
                                {                                    
                                    while (true)
                                    {                                        
                                        innerCts.Token.ThrowIfCancellationRequested();

                                        beginBarrier.SignalAndWait(innerCts.Token);

                                        var oldLeftValue = prevStep.Get(index2 - 1);
                                        var oldValue = prevStep.Get(index2);
                                        var oldRightValue = prevStep.Get(index2 + 1);                                        

                                        var newValue = ApplyRule(oldLeftValue, oldValue, oldRightValue, rule);

                                        nextStep.Set(index2, newValue);                                        

                                        if (newValue || oldValue)
                                        {                                            
                                            lock (boundsSyncObj)
                                            {
                                                if (!leftMostChangedIndex.HasValue || index2 < leftMostChangedIndex.Value)
                                                    leftMostChangedIndex = index2;

                                                if (!rightMostChangedIndex.HasValue || index2 > rightMostChangedIndex.Value)
                                                    rightMostChangedIndex = index2;
                                            }                                            
                                        }

                                        // TODO: cancel not required tasks?                              

                                        endBarrier.SignalAndWait(innerCts.Token);                                        
                                    }
                                }                                
                                finally 
                                {
                                    beginBarrier.RemoveParticipant();                                    
                                    endBarrier.RemoveParticipant();
                                }
                            }, TaskCreationOptions.LongRunning)
                            .ContinueWith(t =>
                            {                                
                                if (!t.IsCanceled)                                
                                    throw t.Exception;
                            }, TaskContinuationOptions.NotOnRanToCompletion));
                        }

                        beginBarrier.SignalAndWait();
                        endBarrier.SignalAndWait();

                        observer.OnNext(nextStep);

                        prevStep = nextStep; 
                    }                    
                },
                ct)                
                .ContinueWith(t =>
                {                    
                    if (t.IsCanceled)
                        observer.OnCompleted();
                    else
                        observer.OnError(t.Exception);                    

                    innerCts.Cancel();
                },
                TaskContinuationOptions.NotOnRanToCompletion);

                return Disposable.Empty;                
            })
            .Publish();
        }
        
        private static bool ApplyRule(bool leftValue, bool value, bool rightValue, BitArray rule)
        {            
            return rule.Get(
                (((leftValue ? 1 : 0)*4) + 
                 ((value ? 1 : 0)*2) + 
                 (rightValue ? 1 : 0)*1));
        }

        private static BitArray GetBitArrayForRule(int ruleNumber)
        {
            if (ruleNumber < 0 || ruleNumber > 255)
                throw new InvalidOperationException("Invalid rule number");

            return new BitArray(new[] { ((byte)ruleNumber) });
        }
    }
}
