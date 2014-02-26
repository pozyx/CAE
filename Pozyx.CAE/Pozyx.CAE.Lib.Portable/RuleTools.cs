using Pozyx.CAE.Lib.CellSpaces;
using System;
using System.Collections;

namespace Pozyx.CAE.Lib
{
    internal static class RuleTools
    {
        public static void ApplyRule(ICellSpace prevStep, ICellSpace nextStep, int index, bool[] rule)
        {
            var oldLeftValue = prevStep.Get(index - 1);
            var oldValue = prevStep.Get(index);
            var oldRightValue = prevStep.Get(index + 1);

            var newValue = ApplyRule(oldLeftValue, oldValue, oldRightValue, rule);

            nextStep.Set(index, newValue);
        }

        private static bool ApplyRule(bool leftValue, bool value, bool rightValue, bool[] rule)
        {
            return rule[
                 (leftValue ? 4 : 0) +
                 (value ? 2 : 0) +
                 (rightValue ? 1 : 0)];
        }

        public static bool[] GetBoolArrayForRule(int ruleNumber)
        {
            if (ruleNumber < 0 || ruleNumber > 255)
                throw new InvalidOperationException("Invalid rule number");

            var bitArray = new BitArray(new[] { ((byte)ruleNumber) });

            var bools = new bool[bitArray.Length];
            ((ICollection)bitArray).CopyTo(bools, 0);

            return bools;
        }
    }
}
