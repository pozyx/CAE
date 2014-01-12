using System;
using System.Collections;
using System.Reactive.Disposables;
using System.Reactive.Linq;
using System.Reactive.Subjects;
using System.Threading;
using System.Threading.Tasks;

namespace Pozyx.CAE.Lib.Runners
{
    public class SingleThreadRunner : IRunner
    {
        private const int BitsInByte = 8;

        public IConnectableObservable<PositionedBitArray> Create(int ruleNumber, CancellationToken ct, Action threadInit = null)
        {            
            var rule = GetBitArrayForRule(ruleNumber);

            return Observable.Create<PositionedBitArray>(observer =>
            {                
                Task.Run(() =>
                {
                    if (threadInit != null)
                        threadInit();
        
                    var prevStep = new PositionedBitArray(new BitArray(1, true), 0);

                    int? leftMostChangedIndex = 0;
                    int? rightMostChangedIndex = 0;

                    while (true)
                    {                        
                        ct.ThrowIfCancellationRequested();

                        //var nextStepLength = prevStep.BitArray.Length + 2; // TODO: optimize
                        //var nextStepOffset = prevStep.Offset - 1; // TODO: optimize                        

                        if (!leftMostChangedIndex.HasValue)
                        {
                            observer.OnCompleted();
                            break;
                        }                        

                        var nextStepLength = rightMostChangedIndex.Value - leftMostChangedIndex.Value + 3;                        
                        var nextStepOffset = leftMostChangedIndex.Value - 1;                        

                        var nextStep = new PositionedBitArray(new BitArray(nextStepLength), nextStepOffset);                        

                        leftMostChangedIndex = null;
                        rightMostChangedIndex = null;

                        for (var index = nextStepOffset; index < nextStepLength + nextStepOffset; index++)
                        {                            
                            var oldLeftValue = prevStep.Get(index - 1);
                            var oldValue = prevStep.Get(index);
                            var oldRightValue = prevStep.Get(index + 1);                            

                            var newValue = ApplyRule(oldLeftValue, oldValue, oldRightValue, rule);                            

                            nextStep.Set(index, newValue);

                            if (!newValue && !oldValue) continue;

                            if (!leftMostChangedIndex.HasValue)                            
                                leftMostChangedIndex = index;                                

                            rightMostChangedIndex = index;
                        }                        

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
