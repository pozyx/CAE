using System;
using System.Collections;

namespace Pozyx.CAE.Lib.CellSpaces
{
    public class BitArrayCellSpace : ICellSpace
    {        
        private BitArray _bitArray;

        public int Offset { get; private set; }

        public int Length
        {
            get { return _bitArray.Length; }
        }

        public void Initialize(BitArray bitArray, int offset)
        {
            _bitArray = bitArray;
            Offset = offset;
        }

        public void Initialize(int length, int offset)
        {
            _bitArray = new BitArray(length);
            Offset = offset;
        }

        public bool Get(int index)
        {
            index -= Offset;

            return
                index >= 0 &&
                index < _bitArray.Length &&
                _bitArray[index];
        }

        public void Set(int index, bool value)
        {
            index -= Offset;

            if (index < 0 || index >= _bitArray.Length)
                throw new Exception("Invalid index to write");

            _bitArray[index] = value;
        }
    }
}
