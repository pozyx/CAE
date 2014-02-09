using Pozyx.CAE.Lib.CellSpaces;
using System;
using System.Collections;

namespace Pozyx.CAE.Lib
{
    internal static class RulesTools
    {
        public static bool ApplyRule(ICellSpace prevStep, ICellSpace nextStep, int index, BitArray rule)
        {
            var oldLeftValue = prevStep.Get(index - 1);
            var oldValue = prevStep.Get(index);
            var oldRightValue = prevStep.Get(index + 1);

            var newValue = ApplyRule(oldLeftValue, oldValue, oldRightValue, rule);

            nextStep.Set(index, newValue);

            return newValue || oldValue;
        }

        private static bool ApplyRule(bool leftValue, bool value, bool rightValue, BitArray rule)
        {
            return rule.Get(
                (((leftValue ? 1 : 0) * 4) +
                 ((value ? 1 : 0) * 2) +
                 (rightValue ? 1 : 0) * 1));
        }

        public static BitArray GetBitArrayForRule(int ruleNumber)
        {
            if (ruleNumber < 0 || ruleNumber > 255)
                throw new InvalidOperationException("Invalid rule number");

            return new BitArray(new[] { ((byte)ruleNumber) });
        }
    }
}
