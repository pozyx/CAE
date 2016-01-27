using System;
using System.Collections;

namespace Pozyx.CAE.Lib.CellSpaces
{
    // one int covers 32 cells
    // backed by array of which length is multiple of tile size 
    //   (requirement for tiled GPU execution)
    public class PaddedPackedIntArrayCellSpace : ICellSpace
    {
        private const int BitsInInt = sizeof(int) * 8;

        // set to multiple of tile size
        private const int PadSize = 1024;

        public int[] PackedCells { get; private set; }

        public int Offset { get; private set; }

        public int Length { get; private set; }

        public void Initialize(BitArray bitArray, int offset)
        {
            Initialize(bitArray.Length, offset);

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
            PackedCells = new int[GetPaddedPackedLength(Length)];
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
                PackedCells[arrayIndex] &= ~(1 << intIndex);
        }

        private int GetPaddedPackedLength(int length)
        {            
            var intLength = (int)Math.Ceiling((double)length / BitsInInt);
            
            return intLength % PadSize == 0 ?
               intLength :
               intLength + PadSize - (intLength % PadSize);
        }

        private void GetPackedIndex(int index, out int arrayIndex, out int intIndex)
        {
            arrayIndex = index / BitsInInt;
            intIndex = index % BitsInInt;
        }
    }
}
