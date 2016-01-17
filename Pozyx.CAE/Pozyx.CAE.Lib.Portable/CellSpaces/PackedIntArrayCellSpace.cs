using System;
using System.Collections;
using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Portable.CellSpaces
{
    public class PackedIntArrayCellSpace : ICellSpace
    {
        public int[] PackedCells { get; private set; }

        public int Offset { get; private set; }

        public int Length { get; private set; }
                
        public void Initialize(BitArray bitArray, int offset)
        {
            Length = bitArray.Length;
            PackedCells = new int[GetPackedLength(Length)];
            Offset = offset;

            for (var i = 0; i < bitArray.Length; i++) 
            {
                int arrayIndex;
                int intIndex;
                GetPackedIndex(i, out arrayIndex, out intIndex);

                if (bitArray[i])
                    PackedCells[arrayIndex] |= (1 << intIndex);
                else
                    PackedCells[arrayIndex] &= ~(1 << intIndex);
            }
        }

        public void Initialize(int length, int offset)
        {
            Length = length;
            PackedCells = new int[GetPackedLength(Length)];
            Offset = offset;
        }

        public bool Get(int index)
        {
            index -= Offset;

            if (index < 0 || index >= Length)
                return false;

            int arrayIndex;
            int intIndex;
            GetPackedIndex(index, out arrayIndex, out intIndex);

            return (PackedCells[arrayIndex] & (1 << intIndex)) != 0;
        }

        public void Set(int index, bool value)
        {
            index -= Offset;

            if (index < 0 || index >= Length)
                throw new Exception("Invalid index to write");

            int arrayIndex;
            int intIndex;
            GetPackedIndex(index, out arrayIndex, out intIndex);

            if (value)
                PackedCells[arrayIndex] |= (1 << intIndex);
            else
                PackedCells[arrayIndex] &=  ~(1 << intIndex);
        }

        private int GetPackedLength(int length)
        {
            return (int) Math.Ceiling((double) length / sizeof(int));
        }

        private void GetPackedIndex(int index, out int arrayIndex, out int intIndex)
        {
            arrayIndex = index / sizeof(int);
            intIndex = index % sizeof(int);
        } 
    }
}
