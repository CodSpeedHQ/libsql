/*
** 2024-07-04
**
** Copyright 2024 the libSQL authors
**
** Permission is hereby granted, free of charge, to any person obtaining a copy of
** this software and associated documentation files (the "Software"), to deal in
** the Software without restriction, including without limitation the rights to
** use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
** the Software, and to permit persons to whom the Software is furnished to do so,
** subject to the following conditions:
**
** The above copyright notice and this permission notice shall be included in all
** copies or substantial portions of the Software.
**
** THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
** IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
** FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
** COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
** IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
** CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
**
******************************************************************************
**
** libSQL basic vector functions
*/
#ifndef SQLITE_OMIT_VECTOR

#include "sqliteInt.h"
#include "vectorInt.h"

#define MAX_FLOAT_CHAR_SZ  1024

/**************************************************************************
** Utility routines for dealing with Vector objects
**************************************************************************/

size_t vectorDataSize(VectorType type, VectorDims dims){
  switch( type ){
    case VECTOR_TYPE_FLOAT32:
      return dims * sizeof(float);
    case VECTOR_TYPE_FLOAT64:
      return dims * sizeof(double);
    default:
      assert(0);
  }
  return 0;
}

void vectorInit(Vector *pVector, VectorType type, VectorDims dims, void *data){
  pVector->type = type;
  pVector->dims = dims;
  pVector->data = data;
  pVector->flags = 0;
}

/*
 * Allocate a Vector object and its data buffer
*/
Vector *vectorAlloc(VectorType type, VectorDims dims){
  void *pVector = sqlite3_malloc(sizeof(Vector) + vectorDataSize(type, dims));
  if( pVector==NULL ){
    return NULL;
  }
  vectorInit(pVector, type, dims, ((char*) pVector) + sizeof(Vector));
  return pVector;
}

/*
** Initialize a static Vector object.
**
** Note that the vector object points to the blob so if
** you free the blob, the vector becomes invalid.
**/
void vectorInitStatic(Vector *pVector, VectorType type, const unsigned char *pBlob, size_t nBlobSize){
  pVector->type = type;
  pVector->flags = VECTOR_FLAGS_STATIC;
  vectorInitFromBlob(pVector, pBlob, nBlobSize);
}

/*
 * Allocate a Vector object and its data buffer from the SQLite context. 
*/
static Vector* vectorContextAlloc(sqlite3_context *context, int type, int dims){
  void *pVector = sqlite3_malloc64(sizeof(Vector) + vectorDataSize(type, dims));
  if( pVector==NULL ){
    sqlite3_result_error_nomem(context);
    return NULL;
  }
  vectorInit(pVector, type, dims, ((char*) pVector) + sizeof(Vector));
  return pVector;
}

/*
 * Free a Vector object and its data buffer allocated, unless the vector is static.
*/
void vectorFree(Vector *pVector){
  if( pVector == NULL ){
    return;
  }
  if( pVector->flags & VECTOR_FLAGS_STATIC ){
    return;
  }
  sqlite3_free(pVector);
}

float vectorDistanceCos(const Vector *pVector1, const Vector *pVector2){
  assert( pVector1->type == pVector2->type );
  switch (pVector1->type) {
    case VECTOR_TYPE_FLOAT32:
      return vectorF32DistanceCos(pVector1, pVector2);
    case VECTOR_TYPE_FLOAT64:
      return vectorF64DistanceCos(pVector1, pVector2);
    default:
      assert(0);
  }
  return 0;
}

float vectorDistanceL2(const Vector *pVector1, const Vector *pVector2){
  assert( pVector1->type == pVector2->type );
  switch (pVector1->type) {
    case VECTOR_TYPE_FLOAT32:
      return vectorF32DistanceL2(pVector1, pVector2);
    case VECTOR_TYPE_FLOAT64:
      return vectorF64DistanceL2(pVector1, pVector2);
    default:
      assert(0);
  }
  return 0;
}

void vectorMult(Vector *pVector, double k){
  switch (pVector->type) {
    case VECTOR_TYPE_FLOAT32:
      vectorF32Mult(pVector, k);
      break;
    case VECTOR_TYPE_FLOAT64:
      vectorF64Mult(pVector, k);
      break;
    default:
      assert(0);
  }
}

void vectorAdd(Vector *v1, const Vector *v2){
  assert( pVector1->type == pVector2->type );
  assert( pVector1->dims == pVector2->dims );
  switch (v1->type) {
    case VECTOR_TYPE_FLOAT32:
      vectorF32Add(v1, v2);
      break;
    case VECTOR_TYPE_FLOAT64:
      vectorF64Add(v1, v2);
      break;
    default:
      assert(0);
  }
}
const char *sqlite3_type_repr(int type){
  switch( type ){
    case SQLITE_NULL:
      return "NULL";
    case SQLITE_INTEGER:
      return "INTEGER";
    case SQLITE_FLOAT:
      return "FLOAT";
    case SQLITE_BLOB:
      return "BLOB";
    case SQLITE_TEXT:
      return "TEXT";
    default:
      return "UNKNOWN";
  }
}
/*
 * Parses vector from text representation (e.g. '[1,2,3]'); vector type must be set
*/
static int vectorParseSqliteText(
  sqlite3_value *arg,
  Vector *pVector,
  char **pzErrMsg
){
  const unsigned char *pzText;
  double elem;
  float *elemsFloat;
  double *elemsDouble;
  int iElem = 0;
  // one more extra character in order to safely print data from elBuf with
  // printf-like method; will be set to zero later
  char valueBuf[MAX_FLOAT_CHAR_SZ + 1];
  int iBuf = 0;

  assert( pVector->type == VECTOR_TYPE_FLOAT32 || pVector->type == VECTOR_TYPE_FLOAT64 );
  assert( sqlite3_value_type(arg) == SQLITE_TEXT );

  if( pVector->type == VECTOR_TYPE_FLOAT32 ){
    elemsFloat = pVector->data;
  } else if( pVector->type == VECTOR_TYPE_FLOAT64 ){
    elemsDouble = pVector->data;
  }

  pzText = sqlite3_value_text(arg);
  if ( pzText == NULL ) return 0;

  while( sqlite3Isspace(*pzText) )
    pzText++;

  if( *pzText != '[' ){
    *pzErrMsg = sqlite3_mprintf("vector: must start with '['");
    goto error;
  }
  pzText++;

  // clear elBuf when we are ready to parse floats
  memset(valueBuf, 0, sizeof(valueBuf));

  for(; *pzText != '\0'; pzText++){
    char this = *pzText;
    if( sqlite3Isspace(this) ){
      continue;
    }
    if( this != ',' && this != ']' ){
      if( iBuf > MAX_FLOAT_CHAR_SZ ){
        *pzErrMsg = sqlite3_mprintf("vector: float string length exceeded %d characters: '%s'", MAX_FLOAT_CHAR_SZ, valueBuf);
        goto error;
      }
      valueBuf[iBuf++] = this;
      continue;
    }
    // empty vector case: '[]'
    if( this == ']' && iElem == 0 && iBuf == 0 ){
      break;
    }
    if( sqlite3AtoF(valueBuf, &elem, iBuf, SQLITE_UTF8) <= 0 ){
      *pzErrMsg = sqlite3_mprintf("vector: invalid float at position %d: '%s'", iElem, valueBuf);
      goto error;
    }
    if( iElem >= MAX_VECTOR_SZ ){
      *pzErrMsg = sqlite3_mprintf("vector: max size exceeded %d", MAX_VECTOR_SZ);
      goto error;
    }
    // clear only first bufidx positions - all other are zero
    memset(valueBuf, 0, iBuf);
    iBuf = 0;
    if( pVector->type == VECTOR_TYPE_FLOAT32 ){
      elemsFloat[iElem++] = elem;
    } else if( pVector->type == VECTOR_TYPE_FLOAT64 ){
      elemsDouble[iElem++] = elem;
    }
    if( this == ']' ){
      break;
    }
  }
  while( sqlite3Isspace(*pzText) )
    pzText++;

  if( *pzText != ']' ){
    *pzErrMsg = sqlite3_mprintf("vector: must end with ']'");
    goto error;
  }
  pzText++;

  while( sqlite3Isspace(*pzText) )
    pzText++;
  
  if( *pzText != '\0' ){
    *pzErrMsg = sqlite3_mprintf("vector: non-space symbols after closing ']' are forbidden");
    goto error;
  }
  pVector->dims = iElem;
  return 0;
error:
  return -1;
}

int vectorParseSqliteBlob(
  sqlite3_value *arg,
  Vector *pVector,
  char **pzErrMsg
){
  switch (pVector->type) {
    case VECTOR_TYPE_FLOAT32: 
      return vectorF32ParseSqliteBlob(arg, pVector, pzErrMsg);
    case VECTOR_TYPE_FLOAT64: 
      return vectorF64ParseSqliteBlob(arg, pVector, pzErrMsg);
    default: 
      assert(0);
  }
  return -1;
}

int detectBlobVectorParameters(sqlite3_value *arg, int *pType, int *pDims, char **pzErrMsg) {
  const u8 *pBlob;
  int nBlobSize;
  
  assert( sqlite3_value_type(arg) == SQLITE_BLOB );

  pBlob = sqlite3_value_blob(arg);
  nBlobSize = sqlite3_value_bytes(arg);
  if( nBlobSize % 2 != 0 ){ 
    // we have trailing byte with explicit type definition
    *pType = pBlob[nBlobSize - 1];
  } else { 
    // else, fallback to FLOAT32
    *pType = VECTOR_TYPE_FLOAT32;
  }
  if( *pType == VECTOR_TYPE_FLOAT32 ){
    *pDims = nBlobSize / sizeof(float);
  } else if( *pType == VECTOR_TYPE_FLOAT64 ){
    *pDims = nBlobSize / sizeof(double);
  } else{
    *pzErrMsg = sqlite3_mprintf("vector: unexpected binary type: got %d, expected %d or %d", *pType, VECTOR_TYPE_FLOAT32, VECTOR_TYPE_FLOAT64);
    return -1;
  }
  if( *pDims > MAX_VECTOR_SZ ){
    *pzErrMsg = sqlite3_mprintf("vector: max size exceeded: %d > %d", *pDims, MAX_VECTOR_SZ);
    return -1;
  }
  return 0;
}

int detectTextVectorParameters(sqlite3_value *arg, int typeHint, int *pType, int *pDims, char **pzErrMsg) {
  const u8 *text;
  int textBytes;
  int iText;
  int textHasDigit = 0;
  
  assert( sqlite3_value_type(arg) == SQLITE_TEXT );
  text = sqlite3_value_text(arg);
  textBytes = sqlite3_value_bytes(arg);
  if( typeHint == 0 ){ 
    *pType = VECTOR_TYPE_FLOAT32;
  }else if( typeHint == VECTOR_TYPE_FLOAT32 ){
    *pType = VECTOR_TYPE_FLOAT32;
  }else if( typeHint == VECTOR_TYPE_FLOAT64 ){
    *pType = VECTOR_TYPE_FLOAT64;
  }else{
    *pzErrMsg = sqlite3_mprintf("unexpected vector type");
    return -1;
  }
  *pDims = 0;
  for(iText = 0; iText < textBytes; iText++){
    if( text[iText] == ',' ){
      *pDims += 1;
    }
    if( sqlite3Isdigit(text[iText]) ){
      textHasDigit = 1;
    }
  }
  if( textHasDigit ){
    *pDims += 1;
  }
  return 0;
}

int detectVectorParameters(sqlite3_value *arg, int typeHint, int *pType, int *pDims, char **pzErrMsg) {
  switch( sqlite3_value_type(arg) ){
    case SQLITE_BLOB:
      return detectBlobVectorParameters(arg, pType, pDims, pzErrMsg);
    case SQLITE_TEXT:
      return detectTextVectorParameters(arg, typeHint, pType, pDims, pzErrMsg);
    default:
      *pzErrMsg = sqlite3_mprintf("vector: unexpected value type: got %s, expected TEXT or BLOB", sqlite3_type_repr(sqlite3_value_type(arg)));
      return -1;
  }
}

int vectorParse(
  sqlite3_value *arg,
  Vector *pVector,
  char **pzErrMsg
){
  switch( sqlite3_value_type(arg) ){
    case SQLITE_BLOB:
      return vectorParseSqliteBlob(arg, pVector, pzErrMsg);
    case SQLITE_TEXT:
      return vectorParseSqliteText(arg, pVector, pzErrMsg);
    default:
      *pzErrMsg = sqlite3_mprintf("vector: unexpected value type: got %s, expected TEXT or BLOB", sqlite3_type_repr(sqlite3_value_type(arg)));
      return -1;
  }
}

void vectorDump(const Vector *pVector){
  switch (pVector->type) {
    case VECTOR_TYPE_FLOAT32:
      vectorF32Dump(pVector);
      break;
    case VECTOR_TYPE_FLOAT64:
      vectorF64Dump(pVector);
      break;
    default:
      assert(0);
  }
}

void vectorMarshalToText(
  sqlite3_context *context,
  const Vector *pVector
){
  switch (pVector->type) {
    case VECTOR_TYPE_FLOAT32:
      vectorF32MarshalToText(context, pVector);
      break;
    case VECTOR_TYPE_FLOAT64:
      vectorF64MarshalToText(context, pVector);
      break;
    default:
      assert(0);
  }
}

void vectorSerialize(
  sqlite3_context *context,
  const Vector *pVector
){
  switch (pVector->type) {
    case VECTOR_TYPE_FLOAT32:
      vectorF32Serialize(context, pVector);
      break;
    case VECTOR_TYPE_FLOAT64:
      vectorF64Serialize(context, pVector);
      break;
    default:
      assert(0);
  }
}

size_t vectorSerializeToBlob(const Vector *pVector, unsigned char *pBlob, size_t nBlobSize){
  switch (pVector->type) {
    case VECTOR_TYPE_FLOAT32:
      return vectorF32SerializeToBlob(pVector, pBlob, nBlobSize);
    case VECTOR_TYPE_FLOAT64:
      return vectorF64SerializeToBlob(pVector, pBlob, nBlobSize);
    default:
      assert(0);
  }
  return 0;
}

size_t vectorDeserializeFromBlob(Vector *pVector, const unsigned char *pBlob, size_t nBlobSize){
  switch (pVector->type) {
    case VECTOR_TYPE_FLOAT32:
      return vectorF32DeserializeFromBlob(pVector, pBlob, nBlobSize);
    case VECTOR_TYPE_FLOAT64:
      return vectorF64DeserializeFromBlob(pVector, pBlob, nBlobSize);
    default:
      assert(0);
  }
  return 0;
}

void vectorInitFromBlob(Vector *pVector, const unsigned char *pBlob, size_t nBlobSize){
  switch (pVector->type) {
    case VECTOR_TYPE_FLOAT32:
      vectorF32InitFromBlob(pVector, pBlob, nBlobSize);
      break;
    case VECTOR_TYPE_FLOAT64:
      vectorF64InitFromBlob(pVector, pBlob, nBlobSize);
      break;
    default:
      assert(0);
  }
}

/**************************************************************************
** SQL function implementations
****************************************************************************/

/*
** Generic vector(...) function with type hint
*/
static void vectorFuncHintedType(
  sqlite3_context *context,
  int argc,
  sqlite3_value **argv,
  int typeHint
){
  char *pzErrMsg = NULL;
  Vector *pVector;
  int type, dims;
  if( argc < 1 ){
    return;
  }
  if( detectVectorParameters(argv[0], typeHint, &type, &dims, &pzErrMsg) != 0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    return;
  }
  pVector = vectorContextAlloc(context, type, dims);
  if( pVector==NULL ){
    return;
  }
  if( vectorParse(argv[0], pVector, &pzErrMsg) != 0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    goto out_free_vec;
  }
  vectorSerialize(context, pVector);
out_free_vec:
  vectorFree(pVector);
}

static void vector32Func(
  sqlite3_context *context,
  int argc,
  sqlite3_value **argv
){
  vectorFuncHintedType(context, argc, argv, VECTOR_TYPE_FLOAT32);
}
static void vector64Func(
  sqlite3_context *context,
  int argc,
  sqlite3_value **argv
){
  vectorFuncHintedType(context, argc, argv, VECTOR_TYPE_FLOAT64);
}

/*
** Implementation of vector_extract(X) function.
*/
static void vectorExtractFunc(
  sqlite3_context *context,
  int argc,
  sqlite3_value **argv
){
  char *pzErrMsg = NULL;
  Vector *pVector;
  unsigned i;
  int type, dims;

  if( argc < 1 ){
    return;
  }
  if( detectVectorParameters(argv[0], 0, &type, &dims, &pzErrMsg) != 0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    return;
  }
  pVector = vectorContextAlloc(context, type, dims);
  if( pVector==NULL ){
    return;
  }
  if( vectorParse(argv[0], pVector, &pzErrMsg)<0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    goto out_free;
  }
  vectorMarshalToText(context, pVector);
out_free:
  vectorFree(pVector);
}

/*
** Implementation of vector_distance_cos(X, Y) function.
*/
static void vectorDistanceCosFunc(
  sqlite3_context *context,
  int argc,
  sqlite3_value **argv
){
  char *pzErrMsg = NULL;
  Vector *pVector1 = NULL, *pVector2 = NULL;
  int type1, type2;
  int dims1, dims2;
  if( argc < 2 ) {
    return;
  }
  if( detectVectorParameters(argv[0], 0, &type1, &dims1, &pzErrMsg) != 0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    goto out_free;
  }
  if( detectVectorParameters(argv[1], 0, &type2, &dims2, &pzErrMsg) != 0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    goto out_free;
  }
  if( type1 != type2 ){
    pzErrMsg = sqlite3_mprintf("vector_distance_cos: vectors must have the same type: %d != %d", type1, type2);
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    goto out_free;
  }
  if( dims1 != dims2 ){
    pzErrMsg = sqlite3_mprintf("vector_distance_cos: vectors must have the same length: %d != %d", dims1, dims2);
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    goto out_free;
  }
  pVector1 = vectorContextAlloc(context, type1, dims1);
  if( pVector1==NULL ){
    goto out_free;
  }
  pVector2 = vectorContextAlloc(context, type2, dims2);
  if( pVector2==NULL ){
    goto out_free;
  }
  if( vectorParse(argv[0], pVector1, &pzErrMsg)<0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    goto out_free;
  }
  if( vectorParse(argv[1], pVector2, &pzErrMsg)<0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    goto out_free;
  }
  sqlite3_result_double(context, vectorDistanceCos(pVector1, pVector2));
out_free:
  if( pVector2 ){
    vectorFree(pVector2);
  }
  if( pVector1 ){
    vectorFree(pVector1);
  }
}

/*
** Implementation of vector_sum(V...) scalar function.
*/
static void vectorSumFunc(
  sqlite3_context *context,
  int argc,
  sqlite3_value **argv
){
  char *pzErrMsg = NULL;
  Vector *pSum = NULL, *pVector = NULL;
  int i;
  int typeSum, dimsSum, typeVector, dimsVector;

  if( argc < 1 ){
    return;
  }
  if( detectVectorParameters(argv[0], 0, &typeSum, &dimsSum, &pzErrMsg) != 0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    goto out_free;
  }
  pSum = vectorContextAlloc(context, typeSum, dimsSum);
  if( pSum == NULL ){
    goto out_free;
  }
  if( vectorParse(argv[0], pSum, &pzErrMsg) < 0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    goto out_free;
  }
  pVector = vectorContextAlloc(context, typeSum, dimsSum);
  if( pVector == NULL ){
    goto out_free;
  }
  for(i = 1; i < argc; i++){
    if( detectVectorParameters(argv[i], 0, &typeVector, &dimsVector, &pzErrMsg) != 0 ){
      sqlite3_result_error(context, pzErrMsg, -1);
      sqlite3_free(pzErrMsg);
      goto out_free;
    }
    if( typeSum != typeVector ){
      pzErrMsg = sqlite3_mprintf("vector_sum: vectors must have the same type: %d != %d", typeSum, typeVector);
      sqlite3_result_error(context, pzErrMsg, -1);
      sqlite3_free(pzErrMsg);
      goto out_free;
    }
    if( dimsSum != dimsVector ){
      pzErrMsg = sqlite3_mprintf("vector_sum: vectors must have the same length: %d != %d", dimsSum, dimsVector);
      sqlite3_result_error(context, pzErrMsg, -1);
      sqlite3_free(pzErrMsg);
      goto out_free;
    }
    if( vectorParse(argv[i], pVector, &pzErrMsg) < 0 ){
      sqlite3_result_error(context, pzErrMsg, -1);
      sqlite3_free(pzErrMsg);
      goto out_free;
    }
    vectorAdd(pSum, pVector);
  }
  vectorSerialize(context, pSum);
out_free:
  if( pSum != NULL ){
    vectorFree(pSum);
  }
  if( pVector != NULL ){
    vectorFree(pVector);
  }
}

struct VectorSumCtx {
  i64 count;
  Vector *pSum;
  Vector *pVector;
};

static void vectorSumAdd(
  sqlite3_context *context,
  int argc,
  sqlite3_value **argv,
  double k
){
  char *pzErrMsg;
  struct VectorSumCtx *p;
  int type, dims;
  assert( argc == 1 );
  UNUSED_PARAMETER(argc);
  p = sqlite3_aggregate_context(context, sizeof(*p));
  if( detectVectorParameters(argv[0], 0, &type, &dims, &pzErrMsg) != 0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    return;
  }
  if( p->count == 0 ){
    p->pSum = vectorContextAlloc(context, type, dims);
    if( p->pSum == NULL ){
      return;
    }
  }
  if( p->pSum->type != type ){
    pzErrMsg = sqlite3_mprintf("vector_sum: vectors must have the same type: %d != %d", p->pSum->type, type);
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    return;
  }
  if( p->pSum->dims != dims ){
    pzErrMsg = sqlite3_mprintf("vector_sum: vectors must have the same length: %d != %d", p->pSum->dims, dims);
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    return;
  }
  if( p->count == 0 ){
    if( vectorParse(argv[0], p->pSum, &pzErrMsg) < 0 ){
      sqlite3_result_error(context, pzErrMsg, -1);
      sqlite3_free(pzErrMsg);
    }else{
      vectorMult(p->pSum, k);
      p->count++;
    }
    return;
  }
  if( p->pVector == NULL ){
    p->pVector = vectorContextAlloc(context, type, dims);
    if( p->pVector == NULL ){
      return;
    }
  }
  if( vectorParse(argv[0], p->pVector, &pzErrMsg) < 0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    return;
  }
  vectorMult(p->pVector, k);
  vectorAdd(p->pSum, p->pVector);
  p->count++;
}

static void vectorSumEnd(sqlite3_context *context, int freeMem){
  struct VectorSumCtx *p;
  p = sqlite3_aggregate_context(context, 0);
  if( p && p->count>0 ){
    vectorSerialize(context, p->pSum);
  }
  if( p && p->pSum != NULL && freeMem ){
    vectorFree(p->pSum);
  }
  if( p && p->pVector != NULL && freeMem ){
    vectorFree(p->pVector);
  }
}

/*
** Implementation of vector_sum aggregate function (step part)
*/
static void vectorSumStep(sqlite3_context *context, int argc, sqlite3_value **argv){
  vectorSumAdd(context, argc, argv, 1.0);
}

/*
** Implementation of vector_sum aggregate function (inverse part)
*/
static void vectorSumInverse(sqlite3_context *context, int argc, sqlite3_value **argv){
  vectorSumAdd(context, argc, argv, -1.0);
}

/*
** Implementation of vector_sum aggregate function (finalize part)
*/
static void vectorSumFinalize(sqlite3_context *context){
  vectorSumEnd(context, 1);
}

/*
** Implementation of vector_sum aggregate function (value part)
*/
static void vectorSumValue(sqlite3_context *context){
  vectorSumEnd(context, 0);
}

/*
** Implementation of vector_mult(V, k) / vector_mult(k, V) function.
*/
static void vectorMultFunc(
  sqlite3_context *context,
  int argc,
  sqlite3_value **argv
){
  char *pzErrMsg;
  sqlite3_value *pMultValue = NULL, *pVectorValue = NULL;
  int type, dims;
  Vector *pVector;
  double k;

  assert( argc == 2 );

  if( sqlite3_value_type(argv[0]) == SQLITE_INTEGER || sqlite3_value_type(argv[0]) == SQLITE_FLOAT ){
    pMultValue = argv[0];
  }
  if( sqlite3_value_type(argv[1]) == SQLITE_INTEGER || sqlite3_value_type(argv[1]) == SQLITE_FLOAT ){
    pMultValue = argv[1];
  }
  if( sqlite3_value_type(argv[0]) == SQLITE_BLOB || sqlite3_value_type(argv[0]) == SQLITE_TEXT ){
    pVectorValue = argv[0];
  }
  if( sqlite3_value_type(argv[1]) == SQLITE_BLOB || sqlite3_value_type(argv[1]) == SQLITE_TEXT ){
    pVectorValue = argv[1];
  }
  if( pMultValue == NULL || pVectorValue == NULL ){
    pzErrMsg = sqlite3_mprintf(
      "vector_mult: unexpected parameters: got %s and %s, but expected vector-compatible and float-compatible types",
      sqlite3_type_repr(sqlite3_value_type(argv[0])),
      sqlite3_type_repr(sqlite3_value_type(argv[1]))
    );
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    return;
  }

  if( detectVectorParameters(pVectorValue, 0, &type, &dims, &pzErrMsg) != 0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    return;
  }
  if( sqlite3_value_type(pMultValue) == SQLITE_INTEGER ){
    k = sqlite3_value_int64(pMultValue);
  }
  if( sqlite3_value_type(pMultValue) == SQLITE_FLOAT ){
    k = sqlite3_value_double(pMultValue);
  }
  pVector = vectorContextAlloc(context, type, dims);
  if( pVector == NULL ){
    return;
  }
  if( vectorParse(pVectorValue, pVector, &pzErrMsg)<0 ){
    sqlite3_result_error(context, pzErrMsg, -1);
    sqlite3_free(pzErrMsg);
    goto out_free;
  }

  vectorMult(pVector, k);
  vectorSerialize(context, pVector);
out_free:
  vectorFree(pVector);
}

/*
 * Marker function which is used in index creation syntax: CREATE INDEX idx ON t(libsql_vector_idx(emb));
*/
static void libsqlVectorIdx(sqlite3_context *context, int argc, sqlite3_value **argv){ 
  // it's important for this function to be no-op as sqlite will apply this function to the column before feeding it to the index
  sqlite3_result_value(context, argv[0]);
}

/*
** Register vector functions.
*/
void sqlite3RegisterVectorFunctions(void){
 static FuncDef aVectorFuncs[] = {
    FUNCTION(vector,              1, 0, 0, vector32Func),
    FUNCTION(vector32,            1, 0, 0, vector32Func),
    FUNCTION(vector64,            1, 0, 0, vector64Func),
    FUNCTION(vector_extract,      1, 0, 0, vectorExtractFunc),
    FUNCTION(vector_sum,         -1, 0, 0, vectorSumFunc),
    FUNCTION(vector_mult,         2, 0, 0, vectorMultFunc),
    FUNCTION(vector_distance_cos, 2, 0, 0, vectorDistanceCosFunc),
    WAGGREGATE(vector_sum,        1, 0, 0, vectorSumStep, vectorSumFinalize, vectorSumFinalize, vectorSumInverse, SQLITE_FUNC_ANYORDER),

    FUNCTION(libsql_vector_idx,  -1, 0, 0, libsqlVectorIdx),
  };
  sqlite3InsertBuiltinFuncs(aVectorFuncs, ArraySize(aVectorFuncs));
}

#endif /* !defined(SQLITE_OMIT_VECTOR) */
