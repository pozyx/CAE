using System;
using System.Collections;
using System.IO;

namespace Pozyx.CAE.Lib
{
    public struct BitArrayCellSpace : ICellSpace
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

        public string ToCellString(long? spaceStart = null)
        {            
            if (!spaceStart.HasValue)
                spaceStart = Offset;

            if (spaceStart > Offset)
                throw new ArgumentException("Space start cannot be higher than offset.", "spaceStart");            

            using (var sw = new StringWriter())
            {
                for (var i = spaceStart.Value; i < Offset; i++)                
                    sw.Write(' ');                

                for (var i = 0; i < _bitArray.Length; i++)
                {
                    //if ((i != 0) && (i % (sizeof(int) * 8) == 0))
                    //    sw.Write(" ");

                    sw.Write(_bitArray[i] ? '█' : ' ');
                }

                return sw.ToString();
            }
        }        
        
        public bool Get(int index)
        {            
            index -= Offset;

            return 
                index >= 0 && 
                index < _bitArray.Length && 
                _bitArray.Get(index);
        }        

        public void Set(int index, bool value)
        {            
            index -= Offset;

            if (index < 0 || index >= _bitArray.Length)
                throw new Exception("Invalid index to write");

            _bitArray.Set(index, value);
        }
    }
}
