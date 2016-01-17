using System;
using System.Collections;
using Pozyx.CAE.Lib.CellSpaces;

namespace Pozyx.CAE.Lib.Portable.CellSpaces
{
    // TODO: use with:
    // 2. C++ CPU runner (for validation and reference)
    // 3. C++ GPU runner (limit concurrency to sizeof(int))
    // 4. tiled gpu runner (create padded cellspace variation)
    public class PackedIntArrayCellSpace : ICellSpace
    {
        private int[] _packedCells;

        public int Offset { get; private set; }

        public int Length { get; private set; }
                
        public void Initialize(BitArray bitArray, int offset)
        {
            Length = bitArray.Length;
            _packedCells = new int[GetPackedLength(Length)];
            Offset = offset;

            for (var i = 0; i < bitArray.Length; i++) 
            {
                int arrayIndex;
                int intIndex;
                GetPackedIndex(i, out arrayIndex, out intIndex);

                if (bitArray[i])
                    _packedCells[arrayIndex] |= (1 << intIndex);
                else
                    _packedCells[arrayIndex] &= ~(1 << intIndex);
            }
        }

        public void Initialize(int length, int offset)
        {
            Length = length;
            _packedCells = new int[GetPackedLength(Length)];
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

            return (_packedCells[arrayIndex] & (1 << intIndex)) != 0;
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
                _packedCells[arrayIndex] |= (1 << intIndex);
            else
                _packedCells[arrayIndex] &=  ~(1 << intIndex);
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
