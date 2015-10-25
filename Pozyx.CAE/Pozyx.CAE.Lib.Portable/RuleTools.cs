using System;
using System.Collections;
using System.Runtime.CompilerServices;
using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib
{
    public static class RuleTools
    {
        internal static void ApplyRule(
            BoolArrayCellSpace inputCellSpace, BoolArrayCellSpace outputCellSpace, bool[] rule, int startIndex, int endIndex)
        {
            var offsetDifference = outputCellSpace.Offset - inputCellSpace.Offset;

            for (int index = startIndex, inputIndex = startIndex + offsetDifference;
                 index < endIndex; 
                 index++, inputIndex++)
            {
                var oldLeftValue = inputIndex - 1 >= 0 && inputIndex - 1 < inputCellSpace.Length && inputCellSpace.Cells[inputIndex - 1];
                var oldValue = inputIndex >= 0 && inputIndex < inputCellSpace.Length && inputCellSpace.Cells[inputIndex];
                var oldRightValue = inputIndex + 1 >= 0 && inputIndex + 1 < inputCellSpace.Length && inputCellSpace.Cells[inputIndex + 1];

                outputCellSpace.Cells[index] = ApplyRule(oldLeftValue, oldValue, oldRightValue, rule);
            }
        }

        internal static void ApplyRule(ICellSpace prevStep, ICellSpace nextStep, int index, bool[] rule)
        {
            var oldLeftValue = prevStep.Get(index - 1);
            var oldValue = prevStep.Get(index);
            var oldRightValue = prevStep.Get(index + 1);

            var newValue = ApplyRule(oldLeftValue, oldValue, oldRightValue, rule);

            nextStep.Set(index, newValue);
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)] 
        private static bool ApplyRule(bool leftValue, bool value, bool rightValue, bool[] rule)
        {
            return rule[
                (leftValue ? 4 : 0) |
                (value ? 2 : 0) |
                (rightValue ? 1 : 0)];
        }

        internal static bool[] GetBoolArrayForRule(int ruleNumber)
        {
            if (ruleNumber < 0 || ruleNumber > 255)
                throw new InvalidOperationException("Invalid rule number");

            var bitArray = new BitArray(new[] { ((byte)ruleNumber) });

            var bools = new bool[bitArray.Length];
            ((ICollection)bitArray).CopyTo(bools, 0);

            return bools;
        }

        public static byte ConvertBitsToByte(bool[] bits)
        {
            if (bits.Length != 8)
                throw new ArgumentException("8 bits expected", nameof(bits));

            var bitArray = new BitArray(bits);
            
            var bytes = new byte[1];
            ((ICollection)bitArray).CopyTo(bytes, 0);
            return bytes[0];
        }
    }
}
